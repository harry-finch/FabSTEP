"""Qualify the planar lip adjoining the last circular wall of the central bore."""

import argparse
import json
from pathlib import Path
import runpy

import numpy as np

rim = runpy.run_path(str(Path(__file__).with_name("qualify-flange-rim.py")))
TOL = rim["TOL"]


def check_triangle_incidence(triangles, stride):
    ends = np.roll(triangles, -1, axis=1)
    if (triangles == ends).any():
        raise ValueError("Collapsed triangle in planar domain")
    keys = (np.minimum(triangles, ends) * stride + np.maximum(triangles, ends)).ravel()
    _, inverse, counts = np.unique(keys, return_inverse=True, return_counts=True)
    forward = np.bincount(inverse.ravel(), weights=(triangles < ends).ravel())
    if (counts > 2).any() or (forward[counts == 2] != 1).any():
        raise ValueError("Invalid internal triangle incidence")


def signed_area(p):
    delta = p - p[0]
    return 0.5 * np.sum(
        delta[:, 0] * np.roll(delta[:, 1], -1) - delta[:, 1] * np.roll(delta[:, 0], -1)
    )


def collar(mesh, plane_id, ring, seams, expected_seam):
    x, y, z = ring["center"]
    center = np.array([y, z])
    radius = ring["radius_mm"]
    ids = np.flatnonzero(mesh.rows["owner"] == plane_id)
    if not 1 <= len(ids) <= 4096:
        raise ValueError("Unexpected collar plane domain")
    check_triangle_incidence(mesh.triangles[ids], mesh.stride)
    points = mesh.rows["points"][ids]
    axial = abs(points[:, :, 0] - x).max()
    if axial > TOL:
        raise ValueError("Collar plane and ring are at different stations")
    loops = list(rim["outline"](mesh, ids))
    if len(loops) != 2:
        raise ValueError("Expected an outside contour and one opening")
    outer = None
    error = None
    outside_owners = None
    for _, ix, near in loops:
        yz = points[ix // 3, ix % 3, 1:]
        if (near >= 0).all():
            if outer is not None or signed_area(yz) <= 0.0:
                raise ValueError("Invalid exterior of collar")
            outer = yz
            outside_owners = np.unique(mesh.rows["owner"][near])
            continue
        labels = [
            seams.get((int(mesh.rows["triangle"][ids[e // 3]]), int(e % 3))) for e in ix
        ]
        if (near >= 0).any() or set(labels) != {expected_seam}:
            raise ValueError("Collar ring does not adjoin the measured outlet")
        delta = yz - center
        u = np.arctan2(delta[:, 1], delta[:, 0])
        step = np.angle(np.exp(1j * (np.roll(u, -1) - u)))
        motion = np.abs(np.linalg.norm(delta, axis=1) - radius)
        if (
            motion.max() > TOL
            or (step >= 0).any()
            or abs(step.sum() + 2 * np.pi) > 1e-10
        ):
            raise ValueError("Ring fails the clockwise source covering")
        error = float(
            np.hypot(
                (motion + 2 * radius * np.sin(step / 4) ** 2).max() + motion.max(),
                axial,
            )
        )
    if outer is None or error is None or error > 0.05:
        raise ValueError("Unresolved collar")
    # The entire disk is inside the polygon, including along its chords.
    offset = outer - center
    d = np.roll(offset, -1, axis=0) - offset
    length2 = (d * d).sum(axis=1)
    if np.any(length2 == 0):
        raise ValueError("Collapsed collar edge")
    t = np.clip(-(offset * d).sum(axis=1) / length2, 0, 1)
    angle = np.arctan2(offset[:, 1], offset[:, 0])
    turn = np.angle(np.exp(1j * (np.roll(angle, -1) - angle)))
    if (
        (turn <= 0).any()
        or abs(turn.sum() - 2 * np.pi) > 1e-10
        or np.linalg.norm(offset + t[:, None] * d, axis=1).min() <= radius + TOL
    ):
        raise ValueError("Exterior does not surround the circular bore once")
    areas = np.cross(points[:, 1] - points[:, 0], points[:, 2] - points[:, 0])[:, 0] / 2
    if (areas <= 0).any():
        raise ValueError("Folded collar face")
    net = signed_area(outer) - np.pi * radius**2
    if net <= 0:
        raise ValueError("Vanishing collar")
    return outer, dict(
        triangles=len(ids),
        outer_edges=len(outer),
        outside_owners=len(outside_owners),
        net_area_mm2=net,
        max_distance_bound_mm=error,
        planar_residual_mm=axial,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    uses = {}
    for line in (args.source / "ring-uses.tsv").read_text().splitlines():
        r, kind, owner = line.split("\t")
        uses.setdefault(int(r), []).append((kind, int(owner)))
    candidates = [
        (i, next(owner for kind, owner in ends if kind == "plane"))
        for i, ends in uses.items()
        if ("torus", 1) in ends and any(k == "plane" for k, _ in ends)
    ]
    ((ring_id, owner),) = candidates
    ring_map = {}
    for line in (args.source / "rings.tsv").read_text().splitlines():
        k, center, axis, radius = line.split("\t")
        ring_map[int(k)] = dict(
            center=json.loads(center), axis=json.loads(axis), radius_mm=float(radius)
        )
    ring = ring_map[ring_id]
    if ring["axis"] != [1.0, 0.0, 0.0]:
        raise ValueError("Unsupported ring axis")
    path = args.source / "planes-3deg.bin"
    record = rim["partition"]["residual"]["RECORD"]
    if (
        not 0 < path.stat().st_size <= 600000 * record.itemsize
        or path.stat().st_size % record.itemsize
    ):
        raise ValueError("Invalid planar dump")
    mesh = rim["boundary"]["DumpMesh"](np.fromfile(path, dtype=record))
    seams = rim["read_seams"](args.source / "analytic-seams.tsv")
    outer, report = collar(mesh, owner, ring, seams, "Torus { index: 1 }")
    for bad in [
        dict(ring, radius_mm=ring["radius_mm"] + 0.02),
        dict(ring, center=np.add(ring["center"], [0.01, 0, 0])),
    ]:
        try:
            collar(mesh, owner, bad, seams, "Torus { index: 1 }")
        except ValueError:
            continue
        raise ValueError("Displaced collar accepted")
    with args.output.open("w") as f:
        print(
            *ring["center"],
            ring["radius_mm"],
            report["net_area_mm2"],
            len(outer),
            file=f,
        )
        np.savetxt(f, outer, fmt="%.17g")
    print(
        json.dumps(dict(**report, ring=ring_id, plane=owner), indent=2, allow_nan=False)
    )


if __name__ == "__main__":
    main()
