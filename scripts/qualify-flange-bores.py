"""Certify five measured chains of periodic faces, including their annular transitions."""

import argparse
from collections import defaultdict
import json
from pathlib import Path
import runpy

import numpy as np

cone = runpy.run_path(str(Path(__file__).with_name("qualify-flange-cones.py")))
require = cone["require"]
TOL = cone["POINT_TOL"]
RECORD = np.dtype([("triangle", "<u4"), ("kind", "<u4"), ("index", "<u4"), ("points", "<f8", (3, 3))])


def chart(samples, wall, rings, neighbors, seams):
    """Full angular coverage and continuous barycentric error on a periodic strip."""
    points, inv = np.unique(samples["points"].reshape(-1, 3), axis=0, return_inverse=True)
    triangles = inv.reshape(-1, 3)
    c = np.asarray(wall["center"])
    p = points - c
    lo, hi = wall["range"]
    require(lo < hi, "Empty wall domain")
    R, r = wall["major"], wall["minor"]
    kind = wall["kind"]
    (ca, ra), (cb, rb) = rings
    if kind == "plane":
        # Annuli may have eccentric rings. At both levels the angle uses the
        # corresponding circle center. Linear interpolation is injective when
        # the increase of radius exceeds the center displacement.
        delta = np.asarray(cb) - ca
        require(ca[0] == cb[0] and np.linalg.norm(delta) < rb - ra, "Crossed annulus")
        radii = np.array([np.linalg.norm(points[:, 1:] - cc[1:], axis=1) for cc in (ca, cb)])
        v = np.argmin(np.abs(radii - np.array([[ra], [rb]])), axis=0).astype(float)
        p = p - v[:, None] * delta
    rho = np.linalg.norm(p[:, 1:], axis=1)
    u = np.arctan2(p[:, 2], p[:, 1])
    if kind == "torus":
        mid = (lo + hi) / 2
        v = mid + np.angle(np.exp(1j * (np.arctan2(p[:, 0], rho - R) - mid)))
    elif kind != "plane":
        v = p[:, 0]
    edges = np.stack([triangles, np.roll(triangles, -1, axis=1)], axis=2).reshape(-1, 2)
    keys = np.min(edges, axis=1) * len(points) + np.max(edges, axis=1)
    unique, start, freq = np.unique(keys, return_counts=True, return_index=True)
    internal = unique[freq == 2]
    require(
        np.all(freq <= 2)
        and np.isin(internal, keys[edges[:, 0] < edges[:, 1]]).all()
        and np.isin(internal, keys[edges[:, 0] > edges[:, 1]]).all(),
        "Invalid interior incidences",
    )
    outer = start[freq == 1]
    levels = np.where(v < (lo + hi) / 2, 0, 1)
    require((levels[edges[outer, 0]] == levels[edges[outer, 1]]).all(), "Broken source boundary")
    sign = -1 if wall["inner"] else 1
    counts = []
    for end, val in enumerate((lo, hi)):
        boundary = outer[levels[edges[outer, 0]] == end]
        a, b = edges[boundary].T
        require(len(a) >= 3 and len(np.unique(a)) == len(a) and set(a) == set(b), "Noncyclic ring")
        v[a] = val
        inc = np.angle(np.exp(1j * (u[b] - u[a])))
        degree = sign * (1 - 2 * end)
        require(np.all(degree * inc > 0) and abs(inc.sum() - degree * 2 * np.pi) < 1e-9, "Boundary degree")
        rc, radius = rings[end]
        require(
            np.hypot(points[a, 0] - rc[0], np.linalg.norm(points[a, 1:] - rc[1:], axis=1) - radius).max()
            <= TOL,
            "Inconsistent measured ring",
        )
        require(
            all(seams.get((int(samples["triangle"][i // 3]), i % 3)) == neighbors[end] for i in boundary),
            "Wrong source incidence",
        )
        counts.append(len(a))
    require(np.all((v >= lo) & (v <= hi)), "Source exits trim interval")
    turn = u[triangles]
    turn = np.angle(np.exp(1j * (turn - turn[:, :1])))
    du = np.ptp(turn, axis=1)
    vv = v[triangles]
    # Consistently signed charts and two primitive opposite boundary windings
    # cover exactly one periodic strip, including across the branch cut.
    det = turn[:, 1] * (vv[:, 2] - vv[:, 0]) - turn[:, 2] * (vv[:, 1] - vv[:, 0])
    require(np.all(du < np.pi / 2) and np.all(sign * det > 1e-12), "Folded wall chart")
    if kind == "torus":
        rr, xx = R + r * np.cos(v), r * np.sin(v)
        kuu, kuv, kvv = R + r, r, r
    else:
        rr = ra + (rb - ra) * (v - lo) / (hi - lo)
        xx = 0.0 if kind == "plane" else v
        kuu, kuv, kvv = max(ra, rb), abs(rb - ra) / (hi - lo), 0.0
    witness = np.hypot(rho - rr, p[:, 0] - xx)
    require(witness.max() <= TOL, "Off-surface vertex")
    dv = np.ptp(vv, axis=1)
    # Taylor remainder of the parametric map over one barycentric triangle.
    # Coordinate variance <= span²/4 and covariance <= span_u*span_v/4.
    # kuu, kuv and kvv bound the norms of its three second derivatives.
    bound = witness[triangles].max(axis=1) + kuu * du**2 / 8 + kuv * du * dv / 4 + kvv * dv**2 / 8
    bound += 512 * np.finfo(float).eps * max(1.0, abs(points).max(), kuu)
    require(bound.max() <= cone["BUDGET"], "Continuous wall deviation")
    return dict(
        triangles=len(samples),
        ring_edges=counts,
        vertex_motion_mm=float(witness.max()),
        bound_mm=float(bound.max()),
    )


def records(path, dtype):
    require(
        0 < path.stat().st_size <= 600000 * dtype.itemsize and path.stat().st_size % dtype.itemsize == 0,
        "Invalid witness table size",
    )
    data = np.fromfile(path, dtype=dtype)
    require(len(np.unique(data["triangle"])) == len(data), "Repeated source triangle")
    return data


def qualify(dump, base, out, negatives=False, shoulder_station=None):
    out.mkdir(parents=True, exist_ok=True)
    rings = {
        int(i): (json.loads(c), float(r))
        for i, c, axis, r in cone["rows"](dump / "rings.tsv")
        if json.loads(axis) == [1.0, 0.0, 0.0]
    }
    uses = defaultdict(list)
    for i, f, j in cone["rows"](dump / "ring-uses.tsv"):
        uses[int(i)].append((f, int(j)))
    cone_table = list(cone["rows"](dump / "cones.tsv"))
    entry_cones = {h["cone_index"] for h in base["bottom_plane"]["holes"]}
    # Only the four small terminal cones have a planar shoulder exit. The
    # opposite rings and every other surface remain the measured Auto values.
    if shoulder_station is not None:
        shifted = []
        for i, _, b, inner in cone_table:
            if int(i) in entry_cones or inner != "true" or not any(k == "plane" for k, _ in uses[int(b)]):
                continue
            center, radius = rings[int(b)]
            require(abs(shoulder_station - center[0]) <= TOL, "Shoulder outside witness tolerance")
            rings[int(b)] = ([shoulder_station, *center[1:]], radius)
            shifted.append(int(b))
        require(len(shifted) == 4, "Expected four terminal cones")
    surfaces = {}
    for family, i, ends, inner, *torus in cone["rows"](dump / "bore-surfaces.tsv"):
        ring_ids = json.loads(ends)
        (ca, ra), (cb, rb) = [rings[e] for e in ring_ids]
        require(ca[1:] == cb[1:], "Off-axis second ring")
        if family == "cylinder":
            require(ra == rb and cb[0] > ca[0], "Invalid cylinder")
            center, R, r, v = ca, ra, 0.0, [0.0, cb[0] - ca[0]]
        else:
            require(family == "torus", "Unsupported surface")
            center, R, r, v = json.loads(torus[0]), float(torus[1]), float(torus[2]), json.loads(torus[3])
            require(center[1:] == ca[1:] and R > r > 0 and 0 < v[1] - v[0] < np.pi, "Unsupported torus")
            for t, (cc, rr) in zip(v, [(ca, ra), (cb, rb)], strict=True):
                require(
                    abs(center[0] + r * np.sin(t) - cc[0]) < 5e-8 and abs(R + r * np.cos(t) - rr) < 5e-8,
                    "Ring off the measured torus",
                )
        surfaces[family, int(i)] = dict(
            kind=family,
            index=int(i),
            inner=inner == "true",
            center=center,
            major=R,
            minor=r,
            range=v,
            rings=ring_ids,
        )
    for i, a, b, inner in cone_table:
        a, b = int(a), int(b)
        (ca, ra), (cb, rb) = rings[a], rings[b]
        require(ca[1:] == cb[1:] and cb[0] > ca[0] and min(ra, rb) > 0 and ra != rb, "Invalid cone")
        surfaces["cone", int(i)] = dict(
            kind="cone",
            index=int(i),
            inner=inner == "true",
            center=ca,
            major=ra,
            minor=0.0,
            range=[0.0, cb[0] - ca[0]],
            rings=[a, b],
        )
    walls = records(dump / "bore-source.bin", RECORD)
    cones = records(dump / "cone-source.bin", cone["RECORD"])
    planar_type = np.dtype(
        [("triangle", "<u4"), ("patch", "<u4"), ("owner", "<u4"), ("points", "<f8", (3, 3))]
    )
    planes = records(dump / "planes-3deg.bin", planar_type)
    all_seams = {}
    for name in ("bore", "cone", "analytic"):
        for t, e, owner in cone["rows"](dump / f"{name}-seams.tsv"):
            k = (int(t), int(e))
            require(k not in all_seams, "Duplicate seam incidence")
            all_seams[k] = cone["owner"](owner)
    # Only the two-circle domains in a single X plane can be annuli. Additional
    # polygonal borders are subsequently ruled out by the whole-owner chart.
    planar_rings = defaultdict(list)
    for i, entries in uses.items():
        for family, idx in entries:
            if family == "plane":
                planar_rings[idx].append(i)
    for idx, ids in planar_rings.items():
        if len(ids) != 2:
            continue
        a, b = sorted(ids, key=lambda i: rings[i][1])
        (ca, ra), (cb, rb) = rings[a], rings[b]
        if ca[0] != cb[0] or np.linalg.norm(np.subtract(ca, cb)) >= rb - ra:
            continue
        sample = planes["points"][planes["owner"] == idx]
        normals = np.cross(sample[:, 1] - sample[:, 0], sample[:, 2] - sample[:, 0])
        require(
            len(sample) > 0 and (normals[:, 1:] == 0).all() and (normals[:, 0] * normals[0, 0] > 0).all(),
            "Inconsistent annulus normals",
        )
        surfaces["plane", idx] = dict(
            kind="plane",
            index=idx,
            center=ca,
            major=ra,
            minor=0.0,
            range=[0.0, 1.0],
            inner=bool(normals[0, 0] > 0),
            rings=[a, b],
        )
    profile = json.loads((dump / "residual-planar.json").read_text())["analytic_owners"]
    result, claims, rejected = [], [], []
    seen = set()
    for hole in base["bottom_plane"]["holes"]:
        prev = -1
        last = ("cone", hole["cone_index"])
        ring = surfaces[last]["rings"][1]
        ca, ra = rings[ring]
        require(
            hole["far_radius_mm"] == ra
            and hole["far_station_mm"] == ca[0]
            and hole["center_yz_mm"] == ca[1:],
            "R-085 shell uses another ring",
        )
        while True:
            (next_wall,) = [k for k in uses[ring] if k != last]
            if next_wall not in surfaces:
                require(next_wall[0] == "plane" and prev >= 0, "Unknown chain termination")
                break
            require(
                next_wall not in seen and next_wall not in {("cone", j) for j in entry_cones},
                "Cycle in chains",
            )
            seen.add(next_wall)
            wall = surfaces[next_wall]
            inlet = wall["rings"].index(ring)
            boundary = [rings[i] for i in wall["rings"]]
            neighbors = []
            for i in wall["rings"]:
                (other,) = [k for k in uses[i] if k != next_wall]
                neighbors.append(other)
            kind, idx = next_wall
            if kind == "plane":
                face = planes[planes["owner"] == idx]
            elif kind == "cone":
                face = cones[cones["cone"] == idx]
            else:
                face = walls[(walls["kind"] == int(kind == "torus")) & (walls["index"] == idx)]
            if kind != "plane":
                require(len(face) == profile[f"{kind.title()} {{ index: {idx} }}"], "Incomplete owner")
            proof = chart(face, wall, boundary, neighbors, all_seams)
            if negatives:
                flipped = face.copy()
                flipped["points"][0] = face["points"][0, ::-1]
                trials = [
                    ("missing_triangle", face[1:], neighbors),
                    ("flipped_triangle", flipped, neighbors),
                    ("exchanged_neighbors", face, neighbors[::-1]),
                ]
                for name, sample, links in trials:
                    try:
                        chart(sample, wall, boundary, links, all_seams)
                    except ValueError:
                        rejected.append((next_wall, name))
                    else:
                        raise ValueError(f"Accepted damaged {wall['kind']} chart: {name}")
            claims.extend((int(i), f"bore-{wall['kind']}-{wall['index']}") for i in face["triangle"])
            result.append(
                dict(**wall, neighbors=neighbors, ring_geometry=boundary, inlet=inlet, previous=prev, **proof)
            )
            prev, last, ring = len(result) - 1, next_wall, wall["rings"][1 - inlet]
    require(
        len(result) == 22 and sum(w["previous"] == -1 for w in result) == 5, "Missing flange chain members"
    )
    require(len(set(i for i, _ in claims)) == len(claims), "Overlapping surfaces")
    (out / "bores.json").write_text(json.dumps(result, indent=2, allow_nan=False) + "\n")
    with (out / "bores.tsv").open("w") as stream:
        for i, name in claims:
            stream.write(f"{i}\t{name}\n")
    with (out / "measurements.txt").open("w") as stream:
        print(len(result), file=stream)
        for w in result:
            (ca, ra), (cb, rb) = w["ring_geometry"]
            print(
                w["kind"],
                w["previous"],
                w["inlet"],
                int(w["inner"]),
                *w["center"],
                w["major"],
                w["minor"],
                *w["range"],
                *ca,
                ra,
                *cb,
                rb,
                file=stream,
            )
    if negatives:
        (out / "refusals.json").write_text(json.dumps(rejected) + "\n")
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("dump", "base", "output"):
        parser.add_argument(name, type=Path)
    parser.add_argument("--check-negatives", action="store_true")
    parser.add_argument("--shoulder-station", type=float)
    args = parser.parse_args()
    result = qualify(
        args.dump, json.loads(args.base.read_text()), args.output, args.check_negatives, args.shoulder_station
    )
    print(json.dumps({k: [w[k] for w in result] for k in ("kind", "triangles", "bound_mm")}))
