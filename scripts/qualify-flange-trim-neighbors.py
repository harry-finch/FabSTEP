"""Check projected ruled triangles on the measured upper edges of the three long flanks."""

import argparse
from collections import Counter
import json
from pathlib import Path
import runpy

import numpy as np
from scipy.spatial import cKDTree

rim = runpy.run_path(str(Path(__file__).with_name("qualify-flange-rim.py")))


def fan(triangle, center, radius, uv, apex):
    # The opposite edge runs in the direction inherited from the source face.
    (u0, x0), (u1, x1) = uv
    du, dx = u1 - u0, x1 - x0
    q, r, p = triangle
    normal = np.cross(r - q, p - q)
    size = np.linalg.norm(normal)
    if not size > 0:
        raise ValueError("Collapsed source triangle")
    normal /= size
    speed = np.hypot(dx, radius * du)
    curvature = radius * du * du
    reserve = (
        512 * np.finfo(float).eps * max(1.0, speed, np.linalg.norm(apex), radius) ** 2
    )

    def curve(t):
        u = u0 + t * du
        point = np.r_[x0 + t * dx, center + radius * np.array([np.cos(u), np.sin(u)])]
        tangent = np.array([dx, -radius * du * np.sin(u), radius * du * np.cos(u)])
        return point, tangent

    a = curve(0.0)[0]
    b = curve(1.0)[0]
    midpoint = curve(0.5)[0]
    # Every candidate triangle point is a convex interpolation between the
    # same apex and its directrix. The linear interpolation remainder on the
    # circle is bounded by sup|E''|/8, also when X changes along the trim.
    motion = max(np.linalg.norm(q - a), np.linalg.norm(r - b))
    bound = max(np.linalg.norm(apex - p), motion + curvature / 8)
    if motion > 1e-5 or bound > 0.05:
        raise ValueError("Measured edge exceeds its source budget")
    # The projected ray around the apex stays in one half-plane. Its signed
    # angular derivative is positive iff this projected Jacobian is positive.
    ends = np.array([a - apex, b - apex])
    ends -= (ends @ normal)[:, None] * normal
    lengths = np.linalg.norm(ends, axis=1)
    if min(lengths) <= reserve:
        raise ValueError("Unresolved apex")
    ray = np.sum(ends / lengths[:, None], axis=0)
    if np.linalg.norm(ray) <= reserve:
        raise ValueError("Unresolved apex")
    ray /= np.linalg.norm(ray)
    if min((a - apex) @ ray, (b - apex) @ ray) <= curvature / 8 + reserve:
        raise ValueError("Angular coverage of fan is ambiguous")
    divisions = [(0.0, 1.0, 0)]
    minimum = np.inf
    while divisions:
        lo, hi, depth = divisions.pop()
        t = (lo + hi) / 2
        e, v = curve(t)
        h = np.cross(v, apex - e) @ normal
        half = (hi - lo) / 2
        # H' = (E'' cross (apex-E)) dot normal.
        lower = (
            h - curvature * (np.linalg.norm(apex - e) + half * speed) * half - reserve
        )
        if lower > 0:
            minimum = min(minimum, lower)
        elif h <= reserve or depth >= 12:
            raise ValueError("Folded projected trim or unresolved sign")
        else:
            divisions.extend([(lo, t, depth + 1), (t, hi, depth + 1)])
    return bound, minimum, abs((midpoint - q) @ normal)


class NativeRim:
    """Anchor a local triangle to the reimported shell by measured endpoints."""

    def __init__(self, native_file, source_file):
        loops = json.loads(native_file.read_text())["loops"]
        self.edges = [
            (loop["id"], i, edge)
            for loop in loops
            for i, edge in enumerate(loop["edges"])
        ]
        self.starts = cKDTree([edge["start_mm"] for _, _, edge in self.edges])
        source_edges = json.loads(source_file.read_text())["edges"]
        self.source_vertices = {
            tuple(e[k]) for e in source_edges for k in ("start_mm", "end_mm")
        }
        self.aggregated_arcs = [
            e
            for _, _, e in self.edges
            if e["kind"] == "circle" and abs(e["signed_sweep_rad"]) > 1.0
        ]

    def location(self, p):
        if tuple(p) not in self.source_vertices:
            return p
        indices = self.starts.query_ball_point(p, 1e-5)
        if indices:
            pts = [np.array(self.edges[i][2]["start_mm"]) for i in indices]
            if any(np.linalg.norm(v - pts[0]) > 5e-8 for v in pts):
                raise ValueError("Ambiguous canonical apex")
            return pts[0]
        hits = []
        for e in self.aggregated_arcs:
            c, axis = np.array(e["center_mm"]), np.array(e["axis"])
            radial = p - c - (axis @ (p - c)) * axis
            if np.linalg.norm(radial) == 0:
                continue
            q = c + radial * e["radius_mm"] / np.linalg.norm(radial)
            start = np.array(e["start_mm"]) - c
            angle = np.arctan2(np.cross(start, radial) @ axis, start @ radial)
            if np.linalg.norm(q - p) <= 1e-5 and 0 < angle / e["signed_sweep_rad"] < 1:
                hits.append(q)
        if len(hits) != 1:
            raise ValueError("Missing canonical apex on a composite arc")
        return hits[0]

    def opposite(self, tri):
        candidates = self.starts.query_ball_point(tri[1], 1e-5)
        matches = [
            self.edges[i]
            for i in candidates
            if np.linalg.norm(tri[0] - self.edges[i][2]["end_mm"]) <= 1e-5
        ]
        if len(matches) != 1:
            raise ValueError("Triangle has no unique opposite edge in the STEP")
        return matches[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("flanks", type=Path)
    parser.add_argument("--native-rim", type=Path, required=True)
    parser.add_argument("--source-rim", type=Path, required=True)
    parser.add_argument("--triangles", type=Path, required=True)
    args = parser.parse_args()
    native = NativeRim(args.native_rim, args.source_rim)
    manifest = []
    dtype = rim["partition"]["residual"]["RECORD"]
    source = args.source / "planes-3deg.bin"
    if (
        not 0 < source.stat().st_size <= 600000 * dtype.itemsize
        or source.stat().st_size % dtype.itemsize
    ):
        raise ValueError("Invalid plane dump")
    rows = np.fromfile(source, dtype=dtype)
    order = np.argsort(rows["triangle"])
    ids = rows["triangle"][order]
    if not (ids[1:] > ids[:-1]).all():
        raise ValueError("Repeated source triangle")
    owners = Counter(map(int, rows["owner"]))
    cases = Counter()
    stats = []
    affected = set()
    domains = json.loads(args.flanks.read_text())["long_flanks"]
    for domain in domains:
        uv = np.asarray(domain["top_uv"])
        uses = domain["top_source_uses"]
        if len(uses) != len(uv) - 1:
            raise ValueError("Incomplete trim provenance")
        for i, (_, _, index, k, owner) in enumerate(uses):
            if owners[owner] != 1:
                cases["outside_single_triangle_domain"] += 1
                continue
            row = rows[order[np.searchsorted(ids, index)]]
            if row["triangle"] != index or row["owner"] != owner:
                raise ValueError("Incorrect neighboring triangle")
            tri = np.roll(row["points"], -k, axis=0)
            pair = uv[i : i + 2]
            # The trace orders the upper edge by angle. Determine the actual
            # traversal of the remaining face from its measured endpoints.
            center = np.asarray(domain["center_yz_mm"])
            radius = domain["radius_mm"]
            u0, x0 = pair[0]
            point = np.r_[x0, center + radius * np.array([np.cos(u0), np.sin(u0)])]
            if np.linalg.norm(point - tri[1]) < np.linalg.norm(point - tri[0]):
                pair = pair[::-1]
            try:
                apex = native.location(tri[2])
                entry = fan(tri, center, radius, pair, apex)
            except ValueError as problem:
                if str(problem) not in (
                    "Angular coverage of fan is ambiguous",
                    "Folded projected trim or unresolved sign",
                    "Unresolved apex",
                ):
                    raise
                cases[str(problem)] += 1
                continue
            loop, num, edge = native.opposite(tri)
            if owner in affected:
                raise ValueError(
                    "A triangle with two curved edges requires another domain"
                )
            manifest.append(
                (
                    owner,
                    loop,
                    num,
                    *edge["start_mm"],
                    *edge["end_mm"],
                    *apex,
                    entry[0],
                    *tri[1],
                    *tri[0],
                    *tri[2],
                )
            )
            stats.append(entry)
            affected.add(owner)
            cases["certified_edges"] += 1
    with args.triangles.open("w") as stream:
        print(len(manifest), file=stream)
        for record in manifest:
            print(*record, file=stream)
    values = np.asarray(stats)
    print(
        json.dumps(
            dict(
                counts=dict(cases),
                unique_single_triangle_owners=len(affected),
                max_local_distance_bound_mm=values[:, 0].max(),
                min_projected_jacobian_bound_mm2=values[:, 1].min(),
                max_curve_midpoint_off_source_plane_mm=values[:, 2].max(),
            ),
            indent=2,
            allow_nan=False,
        )
    )


if __name__ == "__main__":
    main()
