"""Bind the two isolated planar rail ends to their four real, oriented STEP edges."""

import argparse
from collections import defaultdict
import json
from pathlib import Path
import runpy

import numpy as np

rim = runpy.run_path(str(Path(__file__).with_name("qualify-flange-rim.py")))
TOL = rim["TOL"]
NATIVE = 5e-8


def require(ok, message):
    if not ok:
        raise ValueError(message)


def unit(v):
    n = np.linalg.norm(v)
    require(n > 1e-12, "Degenerate direction")
    return v / n


def match_shell(raw, normals, edges, supports):
    """The existing cylindrical face traverses its free edge opposite to the new plane."""
    groups = []
    for i, (a, b) in enumerate(zip(raw, np.roll(raw, -1, axis=0))):
        found = [
            e
            for e in edges
            if e["kind"] == "line"
            and np.linalg.norm(a - e["end_mm"]) <= TOL
            and np.linalg.norm(b - e["start_mm"]) <= TOL
        ]
        require(len(found) == 1, "Missing or ambiguous free coedge")
        (edge,) = found
        owner = supports[edge["face_id"]]
        require(owner["family"] == "cylinder", "Expected a cylindrical neighbor")
        middle = np.asarray(edge["mid_mm"])
        require(
            np.linalg.norm(middle - (np.add(edge["start_mm"], edge["end_mm"]) / 2)) <= NATIVE,
            "Rail is not linear over its full trim",
        )
        delta = middle - owner["axis_origin_mm"]
        radial = delta - np.dot(delta, owner["axis"]) * np.asarray(owner["axis"])
        require(
            abs(np.linalg.norm(radial) - owner["radius_mm"]) <= NATIVE, "Rail outside cylindrical support"
        )
        sense = 1 if owner["orientation"] == "forward" else -1
        require(
            np.dot(normals[i], sense * unit(radial)) >= np.cos(np.deg2rad(3)),
            "Native supporting cylinder disagrees with the adjacent oriented source facet",
        )
        groups.append(edge)
    require(len({e["face_id"] for e in groups}) == 4, "A rail neighbor was counted twice")
    q = np.array([e["end_mm"] for e in groups])
    for i, e in enumerate(groups):
        require(np.linalg.norm(q[(i + 1) % 4] - e["start_mm"]) <= NATIVE, "Native rails have a gap")
    return q, groups


def qualify(mesh, owner, links, native_edges, supports):
    ids = np.flatnonzero(mesh.rows["owner"] == owner)
    require(len(ids) == 2 and len(links) == 4, "Incomplete isolated plane")
    (loop,) = rim["outline"](mesh, ids)
    _, local, near = loop
    require(len(local) == 4 and (near >= 0).all(), "Unexpected source outline")
    raw = mesh.rows["points"][ids[local // 3], local % 3]
    opposite = mesh.rows["points"][near]
    normals = [unit(n) for n in np.cross(opposite[:, 1] - opposite[:, 0], opposite[:, 2] - opposite[:, 0])]
    perimeter = []
    for i, corner in enumerate(local):
        t = int(mesh.rows["triangle"][ids[corner // 3]])
        link = links[(t, corner % 3)]
        near_edge = link["measured_edge"]
        require(
            link["measured_triangle"] == mesh.rows["triangle"][near[i]]
            and link["measured_owner"] == mesh.rows["owner"][near[i]]
            and 0 <= near_edge < 3
            and np.array_equal(opposite[i, near_edge], raw[(i + 1) % 4])
            and np.array_equal(opposite[i, (near_edge + 1) % 3], raw[i])
            and np.array_equal(raw[i], link["start_mm"])
            and np.array_equal(raw[(i + 1) % 4], link["end_mm"]),
            "Source edge attribution changed",
        )
        perimeter.append(link["measured_group"])
    expected = {"long-flank", "short-flank", "upper-tube", "lower-tube"}
    require({g.rsplit("-", 1)[0] for g in perimeter} == expected, "Unexpected four source neighbors")
    q, native = match_shell(raw, normals, native_edges, supports)
    raw_tri = mesh.rows["points"][ids]
    references = {tuple(p): j for j, p in enumerate(raw)}
    corner_ids = np.array([[references[tuple(p)] for p in tri] for tri in raw_tri])
    normal = unit(np.cross(q[1] - q[0], q[2] - q[0]))
    require(np.max(abs((q - q[0]) @ normal)) <= NATIVE, "Nonplanar native rail ends")
    # Both original triangles share their sole interior diagonal in opposite directions.
    directed = np.stack([corner_ids, np.roll(corner_ids, -1, axis=1)], axis=2).reshape(-1, 2)
    keys = 4 * np.min(directed, axis=1) + np.max(directed, axis=1)
    uniq, counts = np.unique(keys, return_counts=True)
    (diagonal,) = uniq[counts == 2]
    require(
        len(uniq) == 5 and np.array_equal(directed[keys == diagonal][0], directed[keys == diagonal][1, ::-1]),
        "Invalid triangulated rectangle",
    )
    adjusted = q[corner_ids]
    vectors = np.cross(adjusted[:, 1] - adjusted[:, 0], adjusted[:, 2] - adjusted[:, 0])
    raw_normals = np.cross(raw_tri[:, 1] - raw_tri[:, 0], raw_tri[:, 2] - raw_tri[:, 0])
    require((vectors @ normal > 0).all() and (raw_normals @ normal > 0).all(), "Incorrect planar winding")
    # Linear interpolation on the same two triangles bounds every source point
    # and covers the entire corrected patch (positive triangles and closed rim).
    motion = np.linalg.norm(raw - q, axis=1).max() + NATIVE
    require(motion <= TOL, "Rail end exceeds source tolerance")
    return dict(
        owner=int(owner),
        triangles=mesh.rows["triangle"][ids].tolist(),
        corners=q.tolist(),
        normal=normal.tolist(),
        area_mm2=float(np.dot(vectors.sum(axis=0), normal) / 2),
        bound_mm=float(motion),
        neighbors=perimeter,
        native_faces=[e["face_id"] for e in native],
    )


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ("source", "source_frontier", "step_frontier", "output"):
        p.add_argument(name, type=Path)
    args = p.parse_args()
    dtype = rim["partition"]["residual"]["RECORD"]
    path = args.source / "planes-3deg.bin"
    require(
        0 < path.stat().st_size <= 600000 * dtype.itemsize and path.stat().st_size % dtype.itemsize == 0,
        "Invalid plane dump",
    )
    mesh = rim["boundary"]["DumpMesh"](np.fromfile(path, dtype=dtype))
    require(
        all(0 < path.stat().st_size <= 16 * 1024**2 for path in (args.source_frontier, args.step_frontier)),
        "Invalid interface report size",
    )
    source = defaultdict(dict)
    for edge in json.loads(args.source_frontier.read_text())["edges"]:
        k = (edge["remaining_triangle"], edge["remaining_edge"])
        group = source[edge["remaining_owner"]]
        require(k not in group, "Repeated source edge")
        group[k] = edge
    shell = json.loads(args.step_frontier.read_text())
    supports = {f["id"]: f for f in shell["faces"]}
    edges = [e for loop in shell["loops"] for e in loop["edges"]]
    groups = []
    for owner, links in source.items():
        if len(links) != 4 or len({e["remaining_triangle"] for e in links.values()}) != 2:
            continue
        if not all(
            e["measured_group"].startswith(("long-flank-", "short-flank-", "lower-tube-", "upper-tube-"))
            for e in links.values()
        ):
            continue
        groups.append(qualify(mesh, owner, links, edges, supports))
    require(len(groups) == 2, "Expected exactly two isolated rail ends")
    for g in groups:
        missing = g["native_faces"][0]
        try:
            qualify(
                mesh, g["owner"], source[g["owner"]], [e for e in edges if e["face_id"] != missing], supports
            )
        except ValueError:
            continue
        raise ValueError("Missing native rail was accepted")
    args.output.mkdir(exist_ok=True, parents=True)
    (args.output / "rail-ends.json").write_text(json.dumps(groups, indent=2, allow_nan=False) + "\n")
    with (args.output / "rail-ends.txt").open("w") as f:
        for group in groups:
            print(group["owner"], *group["normal"], group["area_mm2"], file=f)
            for point, face in zip(group["corners"], group["native_faces"]):
                print(*point, face, file=f)
    print(json.dumps({k: [g[k] for g in groups] for k in ("owner", "area_mm2", "bound_mm")}))


if __name__ == "__main__":
    main()
