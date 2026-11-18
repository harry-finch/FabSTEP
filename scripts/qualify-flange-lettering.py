"""Recover the ten recessed markings from the measured triangles of the Flansch side."""

import argparse
import json
from pathlib import Path
import runpy

import numpy as np

rim = runpy.run_path(str(Path(__file__).with_name("qualify-flange-rim.py")))
TOL = 1e-5


def components(mesh, side_ids, shell_triangles):
    rows = mesh.rows
    owners = rows["owner"]
    adjacent = mesh.neighbors(np.arange(len(rows)), np.arange(3 * len(rows))).reshape(
        -1, 3
    )
    (side_owner,) = np.unique(owners[side_ids])
    loops = list(rim["outline"](mesh, side_ids))
    reached = np.zeros(len(rows), dtype=bool)
    result = []
    for keys, ix, near in loops:
        # The exterior wire surrounds the complete rectangular side. All other
        # wires lie strictly inside it, as checked on the original pierced face.
        if (near < 0).any():
            raise ValueError("Missing planar neighbor of the marked side")
        if np.any(np.isin(rows["triangle"][near], shell_triangles)):
            if not np.isin(rows["triangle"][near], shell_triangles).all():
                raise ValueError(
                    "Marked face only partly adjoins the shell on one wire"
                )
            continue
        seen = np.unique(near)
        if reached[seen].any():
            raise ValueError("Two contours communicate across the relief")
        reached[seen] = True
        chunk = [seen]
        while len(seen):
            neighbors = np.unique(adjacent[seen])
            if neighbors[0] < 0:
                raise ValueError("The relief reaches an unknown surface")
            seen = neighbors[(owners[neighbors] != side_owner) & ~reached[neighbors]]
            if len(seen):
                reached[seen] = True
                chunk.append(seen)
        ids = np.concatenate(chunk)
        if (
            len(ids) > 16000
            or np.isin(rows["triangle"][ids], shell_triangles).any()
            or np.isin(owners, np.unique(owners[ids])).sum() != len(ids)
        ):
            raise ValueError("Relief is oversized or overlaps another source group")
        (boundary,) = rim["outline"](mesh, ids)
        if (
            not np.array_equal(np.sort(boundary[0]), np.sort(keys))
            or (owners[boundary[2]] != side_owner).any()
        ):
            raise ValueError("A marking has an unmatched edge on its opening")
        result.append(ids)
    if len(result) != 10 or len(loops) != 11:
        raise ValueError("Expected ten separate markings")
    return result


def collect(mesh, panel, side_ids, pieces):
    rows = mesh.rows
    ids = np.concatenate(pieces)
    source, inverse = np.unique(
        rows["points"][ids].reshape(-1, 3), axis=0, return_inverse=True
    )
    faces = inverse.reshape(-1, 3)
    side = rows["points"][side_ids].reshape(-1, 3)
    if np.ptp(side[:, 2]) != 0:
        raise ValueError("Expected the source side on a constant Z plane")
    depth = source[:, 2] - side[0, 2]
    if np.min(depth) != 0 or np.max(depth) <= TOL:
        raise ValueError("Unresolved relief depth")
    origin = np.asarray(panel["origin_mm"])
    direction = np.asarray(panel["along_direction"])
    axial = np.array([1.0, 0.0, 0.0])
    normal = np.cross(direction, axial)
    if (
        abs(direction[0]) > 1e-12
        or abs(np.linalg.norm(normal) - 1.0) > 1e-12
        or abs(normal[2]) < 0.999999
    ):
        raise ValueError(
            "Canonical and source supports are not parallel within the local domain"
        )
    # Use the same orthogonal projection as the perforated face at the
    # opening, and restore measured depths on its material side. This is one
    # affine map for every triangle, including the floors of the recesses.
    delta = source - origin
    vertices = (
        origin
        + (delta @ direction)[:, None] * direction
        + (delta @ axial)[:, None] * axial
        + np.sign(normal[2]) * depth[:, None] * normal
    )
    displacement = np.linalg.norm(vertices - source, axis=1).max()
    if displacement > TOL:
        raise ValueError("Excessive motion on the recessed wall")
    groups = []
    max_plane = 0.0
    for owner in np.unique(rows["owner"][ids]):
        local = np.flatnonzero(rows["owner"][ids] == owner)
        tri = vertices[faces[local]]
        area_vectors = np.cross(tri[:, 1] - tri[:, 0], tri[:, 2] - tri[:, 0])
        direction = np.sum(area_vectors, axis=0)
        size = np.linalg.norm(direction)
        if not size > 0:
            raise ValueError("Collapsed face")
        direction /= size
        if np.min(area_vectors @ direction) <= 0.0:
            raise ValueError("Folded face in its own plane")
        residual = abs((tri - tri[0, 0]) @ direction).max()
        max_plane = max(max_plane, residual)
        if residual > 5e-8:
            raise ValueError(f"Planar support outside native tolerance: {residual}")
        loops = []
        positive = []
        reference = tri[0, 0]
        for _, edge, _ in rim["outline"](mesh, ids[local]):
            ring = faces[local[edge // 3], edge % 3]
            loop_points = vertices[ring] - reference
            area = (
                np.cross(loop_points, np.roll(loop_points, -1, axis=0)).sum(axis=0)
                @ direction
                / 2
            )
            loops.append((area, ring))
            positive.append(area > 0)
        if sum(positive) != 1 or abs(sum(area for area, _ in loops) - size / 2) > 1e-9:
            raise ValueError("Face loops do not cover their source triangles")
        loops.sort(key=lambda loop: -loop[0])
        groups.append((int(owner), direction, size / 2, [r for _, r in loops]))
    return (
        vertices,
        groups,
        dict(
            markings=len(pieces),
            triangles=len(ids),
            planar_faces=len(groups),
            max_displacement_mm=displacement,
            max_plane_residual_mm=max_plane,
            max_depth_mm=depth.max(),
            triangles_per_marking=list(map(len, pieces)),
        ),
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ("source", "assembly", "output"):
        parser.add_argument(arg, type=Path)
    args = parser.parse_args()
    path = args.source / "planes-3deg.bin"
    record = rim["partition"]["residual"]["RECORD"]
    size = path.stat().st_size
    if not 0 < size <= 600000 * record.itemsize or size % record.itemsize:
        raise ValueError("Invalid planar dump")
    rows = np.fromfile(path, dtype=record)
    mesh = rim["boundary"]["DumpMesh"](rows)
    shell = []
    side = []
    with (args.assembly / "patches.tsv").open() as stream:
        for line in stream:
            i, label = line.rstrip().split("\t")
            shell.append(int(i))
            if label == "side-plane-decorated":
                side.append(int(i))
    shell_triangles = np.asarray(shell)
    side_ids = np.flatnonzero(np.isin(rows["triangle"], side))
    pieces = components(mesh, side_ids, shell_triangles)
    params = json.loads((args.assembly / "pierced.json").read_text())["pierced_side"]
    vertices, groups, report = collect(mesh, params, side_ids, pieces)
    with args.output.open("w") as f:
        print(len(vertices), len(groups), file=f)
        np.savetxt(f, vertices, fmt="%.17g")
        for owner, direction, area, rings in groups:
            print(owner, *direction, area, len(rings), file=f)
            for ring in rings:
                print(len(ring), *ring, file=f)
    print(json.dumps(report, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
