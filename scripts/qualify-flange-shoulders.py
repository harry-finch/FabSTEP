"""Measure both upper shoulder planes against the assembled rails and bore rings."""

import argparse
import json
from pathlib import Path
import runpy

import numpy as np

collar = runpy.run_path(str(Path(__file__).with_name("qualify-flange-collar.py")))
rim = collar["rim"]
TOL = rim["TOL"]


def at_station(curve, s):
    start, end = np.array(curve["start_mm"]), np.array(curve["end_mm"])
    if curve["kind"] == "line":
        return start + (end - start) * s / np.linalg.norm(end - start)
    if curve["kind"] != "circle":
        raise ValueError("Unsupported shoulder trim")
    center = np.array(curve["center_mm"])
    delta = start - center
    a = s / curve["radius_mm"]
    return center + delta * np.cos(a) + np.cross(curve["axis"], delta) * np.sin(a)


def outside(raw, uses, curves, station):
    # Start inside an unshared chain, so no shared arc straddles the array seam.
    first = next(i for i, u in enumerate(uses) if u is None)
    raw = np.roll(raw, -first, axis=0)
    uses = uses[first:] + uses[:first]
    vertices = raw.copy()
    vertices[:, 0] = station
    changed = {}
    groups = []
    for i, use in enumerate(uses):
        if use is None:
            groups.append((i, i + 1, None))
            continue
        index, s0, s1 = use
        for k, s in [(i, s0), ((i + 1) % len(raw), s1)]:
            point = at_station(curves[index], s)
            if (
                abs(point[0] - station) > 5e-8
                or np.linalg.norm(point - raw[k]) > TOL
                or (k in changed and np.linalg.norm(point - changed[k]) > 5e-8)
            ):
                raise ValueError("Source rim misses the shared rail")
            changed[k] = point
            vertices[k] = point
        if i and uses[i - 1] is not None and uses[i - 1][0] == index:
            if (
                abs(uses[i - 1][2] - s0) > 5e-8
                or (s1 - s0) * (uses[i - 1][2] - uses[i - 1][1]) <= 0
            ):
                raise ValueError("Discontinuous rail covering")
            groups[-1] = (groups[-1][0], i + 1, index)
        else:
            groups.append((i, i + 1, index))
    trims = []
    area = 0.0
    max_sag = 0.0
    origin = vertices[0, 1:]
    for i, j, ix in groups:
        a, b = vertices[[i, j % len(raw)], 1:]
        center = [0.0, 0.0]
        sweep = 0.0
        if ix is not None and curves[ix]["kind"] == "circle":
            curve = curves[ix]
            if abs(abs(curve["axis"][0]) - 1.0) > 1e-12:
                raise ValueError("Arc leaves shoulder plane")
            r = curve["radius_mm"]
            center = curve["center_mm"][1:]
            sweep = curve["axis"][0] * (uses[j - 1][2] - uses[i][1]) / r
            steps = np.array([s1 - s0 for _, s0, s1 in uses[i:j]]) / r
            max_sag = max(max_sag, float(np.max(2 * r * np.sin(steps / 4) ** 2)))
            area += r * r * (sweep - np.sin(sweep)) / 2
        u, v = a - origin, b - origin
        area += (u[0] * v[1] - u[1] * v[0]) / 2
        trims.append((int(sweep != 0), int(ix is not None), *a, *b, *center, sweep))
    motion = np.linalg.norm(vertices - raw, axis=1).max()
    if area <= 0 or motion + max_sag > 0.05:
        raise ValueError("Shoulder exterior exceeds source tolerance")
    return trims, area, motion + max_sag


def measure(mesh, owner, holes, seams, uses, curves, station):
    ids = np.flatnonzero(mesh.rows["owner"] == owner)
    if not 0 < len(ids) < 4096:
        raise ValueError("Unexpected shoulder size")
    collar["check_triangle_incidence"](mesh.triangles[ids], mesh.stride)
    points = mesh.rows["points"][ids]
    axial = abs(points[:, :, 0] - station).max()
    tri_areas = (
        np.cross(points[:, 1] - points[:, 0], points[:, 2] - points[:, 0])[:, 0] / 2
    )
    if axial > TOL or (tri_areas <= 0).any():
        raise ValueError("Shoulder is not a consistently oriented plane")
    rings = {}
    outer = None
    bound = axial
    for _, local, near in rim["outline"](mesh, ids):
        refs = list(zip(mesh.rows["triangle"][ids[local // 3]], local % 3))
        raw = points[local // 3, local % 3]
        if (near >= 0).all():
            if outer is not None:
                raise ValueError("Several exterior contours")
            outer = raw[:, 1:]
            trims, area, bound = outside(
                raw, [uses.get(k) for k in refs], curves, station
            )
            continue
        tags = {seams.get(k) for k in refs}
        if (near >= 0).any() or len(tags) != 1:
            raise ValueError("Mixed bore owners")
        (tag,) = tags
        if tag not in holes or tag in rings:
            raise ValueError("Unexpected cone adjoining the shoulder")
        center_xyz, radius = holes[tag]
        if abs(center_xyz[0] - station) > TOL:
            raise ValueError("Bore station differs from the shoulder")
        center = np.array(center_xyz[1:])
        delta = raw[:, 1:] - center
        angle = np.arctan2(delta[:, 1], delta[:, 0])
        turn = np.angle(np.exp(1j * (np.roll(angle, -1) - angle)))
        motion = abs(np.linalg.norm(delta, axis=1) - radius).max()
        if motion > TOL or (turn >= 0).any() or abs(turn.sum() + 2 * np.pi) > 1e-10:
            raise ValueError("Bore does not cover its ring once")
        rings[tag] = (*center, radius)
        ring_bound = np.hypot(motion + 2 * radius * np.sin(turn / 4) ** 2, axial).max()
        if ring_bound > 0.05:
            raise ValueError("Bore exceeds tolerance")
    if outer is None or set(rings) != set(holes) or len(rings) != 2:
        raise ValueError("Incomplete shoulder")
    parsed = np.array(list(rings.values()))
    if np.linalg.norm(parsed[0, :2] - parsed[1, :2]) <= parsed[:, 2].sum() + 2 * TOL:
        raise ValueError("Overlapping shoulder openings")
    for y, z, radius in rings.values():
        q = outer - [y, z]
        d = np.roll(q, -1, axis=0) - q
        t = np.clip(-(q * d).sum(axis=1) / (d * d).sum(axis=1), 0, 1)
        angle = np.arctan2(q[:, 1], q[:, 0])
        if (
            np.linalg.norm(q + t[:, None] * d, axis=1).min() <= radius + bound + TOL
            or abs(
                np.angle(np.exp(1j * (np.roll(angle, -1) - angle))).sum() - 2 * np.pi
            )
            > 1e-10
        ):
            raise ValueError("A hole crosses the shoulder exterior")
        area -= np.pi * radius**2
    if area <= 0 or sum(t[1] for t in trims) != 5:
        raise ValueError("Incomplete measured shared perimeter")
    return (owner, station, area, len(trims)), list(rings.values()), trims


def validate_map(groups, curves):
    # Some rails meet more than one original owner. Prove their whole chain
    # before selecting the intervals belonging to a particular shoulder.
    if sorted(g["native_edge"] for g in groups) != list(range(len(curves))):
        raise ValueError("Missing or repeated native rail")
    for group in groups:
        curve = curves[group["native_edge"]]
        length = (
            np.linalg.norm(np.subtract(curve["end_mm"], curve["start_mm"]))
            if curve["kind"] == "line"
            else curve["radius_mm"] * curve["sweep_rad"]
        )
        steps = np.asarray(
            [(edge["start_mm"], edge["end_mm"]) for edge in group["edges"]]
        )
        if not len(steps) or not np.isfinite(steps).all():
            raise ValueError("Empty or invalid native rail")
        if steps[0, 0] > steps[-1, 1]:
            steps = steps[::-1, ::-1]
        if (
            abs(steps[0, 0]) > 5e-8
            or abs(steps[-1, 1] - length) > 5e-8
            or (steps[:, 1] <= steps[:, 0]).any()
            or not np.allclose(steps[:-1, 1], steps[1:, 0], rtol=0, atol=5e-8)
        ):
            raise ValueError("Incomplete or overlapping native rail covering")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("source", "interface_map", "native_contour", "output"):
        parser.add_argument(name, type=Path)
    args = parser.parse_args()
    uses = {}
    curves = json.loads(args.native_contour.read_text())["edges"]
    groups = json.loads(args.interface_map.read_text())
    validate_map(groups, curves)
    for group in groups:
        for entry in group["edges"]:
            k = tuple(entry["outside"])
            if k in uses:
                raise ValueError("Duplicate interface claim")
            uses[k] = (group["native_edge"], entry["end_mm"], entry["start_mm"])
    source = args.source / "planes-3deg.bin"
    dtype = rim["partition"]["residual"]["RECORD"]
    if (
        not 0 < source.stat().st_size <= 600000 * dtype.itemsize
        or source.stat().st_size % dtype.itemsize
    ):
        raise ValueError("Invalid planar dump")
    mesh = rim["boundary"]["DumpMesh"](np.fromfile(source, dtype=dtype))
    seams = rim["read_seams"](args.source / "analytic-seams.tsv")
    rings = {}
    for line in (args.source / "rings.tsv").read_text().splitlines():
        key, center, axis, radius = line.split("\t")
        rings[int(key)] = (json.loads(center), json.loads(axis), float(radius))
    attached = {}
    for line in (args.source / "ring-uses.tsv").read_text().splitlines():
        key, kind, owner = line.split("\t")
        attached.setdefault(int(key), []).append((kind, int(owner)))
    blocks = []
    for owner in (3, 21957):
        holes = {}
        for key, neighbors in attached.items():
            if ("plane", owner) in neighbors:
                ((_, cone),) = [v for v in neighbors if v[0] == "cone"]
                center, axis, radius = rings[key]
                if axis != [1.0, 0.0, 0.0]:
                    raise ValueError("Unexpected bore axis")
                holes[f"Cone {{ index: {cone} }}"] = (center, radius)
        blocks.append(measure(mesh, owner, holes, seams, uses, curves, 7.0))
    with args.output.open("w") as stream:
        for header, holes, trims in blocks:
            for row in (header, *holes, *trims):
                print(*row, file=stream)
            print(
                "plane",
                header[0],
                "net area",
                header[2],
                "outer edges",
                len(trims),
                "holes",
                len(holes),
            )


if __name__ == "__main__":
    main()
