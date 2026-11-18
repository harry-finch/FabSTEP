"""Build source references and triangle claims for the measured open Flansch shell.

Run with the directory of the optional R-080 dump, and a local output directory.
All coordinates and source references produced by this laboratory are private.
"""
import argparse
import json
from pathlib import Path
import runpy

import numpy as np

root = Path(__file__).parent
upper = runpy.run_path(str(root / "qualify-flange-upper-tubes.py"))
rim = upper["rim"]
flanks = upper["flank_tools"]
pierced = runpy.run_path(str(root / "qualify-flange-pierced-side.py"))
cones = runpy.run_path(str(root / "qualify-flange-cones.py"))


def write_json(path, data):
    path.write_text(json.dumps(data, separators=(",", ":"), allow_nan=False) + "\n")


def interface_loops(rows, samples, claims, seams, cone_seams, contour_file):
    # Reconstruct directed incidence only, using the same analyzed vertices.
    # Plane and cone dumps omit the cylinders and full tori. Their incidences
    # are resolved by the additional source-edge table from the Rust probe.
    cone_rows = np.zeros(len(samples), dtype=rows.dtype)
    for key in ("triangle", "points"):
        cone_rows[key] = samples[key]
    cone_rows["owner"] = samples["cone"]
    combined = np.concatenate([rows, cone_rows])
    if (len(combined) > 600000 or combined["triangle"].max() >= 600000
            or len(np.unique(combined["triangle"])) != len(combined)):
        raise ValueError("Invalid or duplicate analysis triangle indices")
    mesh = rim["boundary"]["DumpMesh"](combined)
    tags = np.full(int(combined["triangle"].max()) + 1, -1, dtype=int)
    names = list(claims)
    for i, name in enumerate(names):
        ids = claims[name]
        if np.any(tags[ids] >= 0) or len(np.unique(ids)) != len(ids):
            raise ValueError("Overlapping source claims")
        tags[ids] = i
    group = tags[combined["triangle"]]
    ids = np.flatnonzero(group >= 0)
    loops = []
    main_contour = None
    for _, local_edges, outside in rim["outline"](mesh, ids):
        inside = ids[local_edges // 3]
        groups = sorted({names[i] for i in group[inside]})
        neighbors = {}
        ordered_owners = []
        for row, edge, other in zip(inside, local_edges % 3, outside, strict=True):
            if other >= 0:
                if group[other] >= 0:
                    raise ValueError("A free edge points to another claimed triangle")
                kind = "Plane" if other < len(rows) else "Cone"
                owner = (kind, int(combined["owner"][other]))
            else:
                table = seams if row < len(rows) else cone_seams
                label = table[int(combined["triangle"][row]), int(edge)]
                kind, number = cones["owner"](label)
                owner = (kind.title(), number)
            neighbors[owner] = neighbors.get(owner, 0) + 1
            ordered_owners.append(owner)
        if len(groups) > 1:
            points = combined["points"][inside, local_edges % 3]
            if main_contour is not None:
                raise ValueError("Several principal interface contours")
            if (outside < 0).any():
                raise ValueError("Main contour lacks an opposite source triangle")
            local = local_edges % 3
            ends = mesh.triangles[inside, (local + 1) % 3]
            opposite = mesh.triangles[outside] == ends[:, None]
            if not np.all(opposite.sum(axis=1) == 1):
                raise ValueError("Ambiguous opposite source edge")
            other_local = opposite.argmax(axis=1)
            if not np.array_equal(mesh.triangles[outside, (other_local+1) % 3], mesh.triangles[inside, local]):
                raise ValueError("Opposite source edge does not reverse the contour")
            main_contour = dict(points_mm=points.tolist(), neighbors=ordered_owners,
                                inside_uses=np.column_stack([combined["triangle"][inside], local]).tolist(),
                                outside_uses=np.column_stack([combined["triangle"][outside], other_local]).tolist())
        loops.append(dict(source_edges=len(inside), faces=groups,
                          outside_owners=len(neighbors),
                          outside_kinds=sorted({kind for kind, _ in neighbors})))
    count_cones = sum(len(lp["faces"]) == 1 and lp["faces"][0].startswith("cone-") for lp in loops)
    count_cutouts = sum(lp["faces"] == ["side-plane-decorated"] for lp in loops)
    if main_contour is None or (len(loops), count_cones, count_cutouts) != (16, 5, 10):
        raise ValueError("Source free loops differ from the native assembly")
    write_json(contour_file, main_contour)
    return loops


def assemble(dump, out, negative):
    out.mkdir(parents=True, exist_ok=True)
    source = dump / "planes-3deg.bin"
    record = rim["partition"]["residual"]["RECORD"]
    length = source.stat().st_size
    if not 0 < length <= 600000 * record.itemsize or length % record.itemsize:
        raise ValueError("Invalid planar dump length")
    seams = rim["read_seams"](dump / "analytic-seams.tsv")
    rows = np.fromfile(source, dtype=record)
    reports = {}

    def visit(mesh, supports, patches, labels, incidences):
        groups = {}
        args = (mesh, supports, patches, labels, incidences)
        # First identify the actual holes by incidence, then use the exact rings
        # from Auto to recheck the plane that will be passed to OCCT.
        profile = rim["qualify"](*args, seams=seams)
        bore_report = cones["qualify"](dump, profile)
        bores = {b["cone_index"]: b for b in bore_report["cones"]}
        reports["bores"] = [{k: b[k] for k in ("cone_index", "triangles", "boundary_edges",
                                               "vertex_motion_mm", "distance_bound_mm")}
                            for b in bores.values()]
        reports["base"] = rim["qualify"](*args, out/"base.json", seams, bores, groups)
        belt = json.loads((out/"base.json").read_text())
        reports["flanks"] = flanks["qualify"](*args, out/"flanks.json", negative, belt, groups)
        short = json.loads((out/"flanks.json").read_text())["short_flanks"]
        reports["upper_tubes"], tubes = upper["qualify"](mesh, patches, labels, belt, short, groups)
        reports["pierced_side"], side = pierced["qualify"](mesh, patches, labels, belt, short, negative, groups)
        write_json(out/"upper.json", tubes)
        write_json(out/"pierced.json", side)

        claims = {name: rows["triangle"][ids] for name, ids in groups.items()}
        samples = np.fromfile(dump/"cone-source.bin", dtype=cones["RECORD"])
        for i, bore in bores.items():
            ids = samples["triangle"][samples["cone"] == i]
            if len(ids) != bore["triangles"]:
                raise ValueError("Incomplete cone claim")
            claims[f"cone-{i}"] = ids
        if len(claims) != 34:
            raise ValueError("Expected 34 complete source groups")
        reports["interfaces"] = interface_loops(rows, samples, claims, seams, rim["read_seams"](dump/"cone-seams.tsv"),
                                              out/"source-contour.json")
        counts = {name: len(ids) for name, ids in claims.items()}
        write_json(out/"counts.json", counts)
        with (out/"patches.tsv").open("w") as stream:
            for name, ids in claims.items():
                stream.writelines(f"{i}\t{name}\n" for i in ids)

    rim["partition"]["qualify"](rows, visit_partition=visit)
    write_json(out/"reports.json", reports)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dump", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--check-negatives", action="store_true")
    args = parser.parse_args()
    assemble(args.dump, args.output, args.check_negatives)
