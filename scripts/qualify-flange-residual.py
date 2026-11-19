"""Build a source-indexed witness for all triangles adjoining a qualified open shell."""

import argparse
from collections import Counter
import json
from pathlib import Path
import runpy

import numpy as np

lab = runpy.run_path(str(Path(__file__).with_name("qualify-flange-remainder.py")))
require = lab["require"]


def prepare(folder, output):
    ends = json.loads((folder / "rail-ends/rail-ends.json").read_text())
    old, nplanes, analytic, hist, groups, names, *_ = lab["attribution"](
        folder / "source",
        folder / "assembly",
        folder / "bores",
        folder / "lettering.txt",
        ends,
    )
    pts, tri, pairs = lab["read_incidence"](folder / "source", old, nplanes, analytic)
    trims = lab["trim_ids"](
        lab["trim_rows"](folder / "trim-triangles.txt"),
        old,
        nplanes,
        hist,
        groups,
        pts,
        tri,
    )
    for i in trims:
        groups[i] = len(names)
        names.append(f"trim-{old[i]}")
    _, xyz, a, b, rows = lab["interface_report"](pts, tri, pairs, groups, old, names)
    native = json.loads((folder / "trim-frontier/frontier.json").read_text())
    matched = []
    lab["match_native"](native, xyz, a, b, rows, matched)
    ids = np.flatnonzero(groups < 0)
    used, local = np.unique(tri[ids], return_inverse=True)
    local = local.reshape(-1, 3)
    index = np.full(len(pts), -1)
    index[used] = np.arange(len(used))
    curved_per_triangle = Counter()
    with output.open("w") as stream:
        source = pts[tri] - pts[0]
        volume = (
            np.einsum(
                "ij,ij->i", source[:, 0], np.cross(source[:, 1], source[:, 2])
            ).sum()
            / 6
        )
        area = (
            np.linalg.norm(
                np.cross(source[:, 1] - source[:, 0], source[:, 2] - source[:, 0]),
                axis=1,
            ).sum()
            / 2
        )
        print(
            len(used),
            len(ids),
            len(matched),
            "%.17g" % volume,
            "%.17g" % area,
            file=stream,
        )
        np.savetxt(stream, [pts.min(axis=0), pts.max(axis=0)], fmt="%.17g")
        np.savetxt(stream, pts[used], fmt="%.17g")
        np.savetxt(stream, np.column_stack((ids, local)), fmt="%d")
        for edge, path in matched:
            vertices = []
            for j in path:
                row = rows[j]
                t, k = row["remaining_triangle"], row["remaining_edge"]
                require(0 <= k < 3, "Invalid source corner")
                start, end = index[tri[t, (k + 1) % 3]], index[tri[t, k]]
                require(not vertices or vertices[-1] == start, "Broken source path")
                vertices.extend([end] if vertices else [start, end])
                if edge["kind"] != "line":
                    curved_per_triangle[t] += 1
            print(
                edge["face_id"],
                len(vertices),
                *vertices,
                *["%.17g" % x for x in (*edge["start_mm"], *edge["end_mm"])],
                file=stream,
            )
    return dict(
        vertices=len(used),
        triangles=len(ids),
        native_edges=len(matched),
        measured_volume_mm3=volume,
        curved_edges_per_triangle=dict(Counter(curved_per_triangle.values())),
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("checkpoint", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    print(json.dumps(prepare(args.checkpoint, args.output), indent=2))
