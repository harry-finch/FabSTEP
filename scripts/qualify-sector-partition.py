"""Find bounded completions of the two flange bands using whole unassigned faces."""
import argparse
from itertools import combinations
import json
from pathlib import Path
import runpy

import numpy as np

boundary = runpy.run_path(str(Path(__file__).with_name("qualify-sector-boundaries.py")))
residual = boundary["residual"]
TOL = residual["TOLERANCE"]


def rim(mesh, ids):
    triangles = mesh.triangles[ids]
    keys = boundary["edge_key"](triangles, np.roll(triangles, -1, axis=1), mesh.stride).ravel()
    _, first, count = np.unique(keys, return_index=True, return_counts=True)
    return mesh.neighbors(ids, first[count == 1])


def match(mesh, ids, center, major, minor, axial_center, labels):
    rows = mesh.rows
    neighbors = rim(mesh, ids)
    neighbors = neighbors[neighbors >= 0]
    neighbors = neighbors[labels[neighbors] < 0]
    pts = rows["points"][neighbors]
    rho = np.linalg.norm(pts[:, :, 1:] - center, axis=2)
    error = np.abs(np.hypot(pts[:, :, 0] - axial_center, rho - major) - minor)
    possible = (error.max(axis=1) <= TOL) & (np.ptp(pts[:, :, 0], axis=1) > TOL)
    owners = np.unique(rows["owner"][neighbors[possible]])
    if len(owners) > 5:
        raise ValueError("More than five possible neighboring owners")
    options = []
    for number in range(1, len(owners) + 1):
        for extension in combinations(owners, number):
            added = np.flatnonzero(np.isin(rows["owner"], extension))
            if np.any(labels[added] >= 0):
                continue
            merged = np.r_[ids, added]
            report = boundary["chart"](mesh, merged, center, major, minor, axial_center)
            if report["admissible"]:
                options.append((merged, report))
    # Restrict conclusions to unique completions in this one-ring search.
    if len(options) != 1:
        raise ValueError(f"Found {len(options)} admissible completions")
    return options[0]


def axial_neighbors(mesh, supports, patches, final):
    """Track actual side walls leaving the low band towards increasing X."""
    points = mesh.rows["points"]
    stations = sorted({support[-1] for support in supports})
    if len(stations) != 2:
        raise ValueError("Expected two separate meridian center stations")
    summary = []
    for tag, (_, center, major, minor, xc) in enumerate(supports):
        if xc != stations[0]:
            continue
        neighbors = rim(mesh, patches[tag][0])
        outside = neighbors[neighbors >= 0]
        seeds = np.unique(outside[(points[outside, :, 0].min(axis=1) >= xc - TOL)
                                  & (points[outside, :, 0].max(axis=1) > xc + TOL)])
        if not len(seeds):
            continue
        rho = np.linalg.norm(points[:, :, 1:] - center, axis=2)
        radius = min([major - minor, major + minor],
                     key=lambda r: np.abs(rho[seeds] - r).max())
        matches = ((np.abs(rho - radius).max(axis=1) <= TOL)
                   & (points[:, :, 0].min(axis=1) >= xc - TOL)
                   & (np.ptp(points[:, :, 0], axis=1) > TOL) & (final < 0))
        matches &= ~np.isin(mesh.rows["owner"], mesh.rows["owner"][~matches])
        row = dict(support=tag, seeds=len(seeds), eligible_seeds=int(matches[seeds].sum()), walls=[])
        if matches[seeds].all():
            for component in boundary["components_at_seeds"](mesh, np.flatnonzero(matches), seeds):
                other = rim(mesh, component)
                upper = final[other[other >= 0]]
                contacts = {str(i): int((upper == i).sum()) for i, support in enumerate(supports)
                            if support[-1] == stations[1] and (upper == i).any()}
                row["walls"].append(dict(triangles=len(component), owners=len(np.unique(mesh.rows["owner"][component])),
                                         x_range_mm=[float(points[component, :, 0].min()),
                                                     float(points[component, :, 0].max())],
                                         upper_edges=contacts))
        summary.append(row)
    return summary


def common_meridians(mesh, supports, patches, incidences, out_path):
    """Re-estimate three tangent guide circles with two exact external contacts."""
    from scipy.optimize import least_squares

    adjacent = {a: [b for (i, b) in incidences if i == a] for a, _ in incidences}
    roots = [a for a, others in adjacent.items() if len(others) == 2]
    if len(roots) != 1 or len(adjacent) != 3:
        raise ValueError("Expected one three-sector chain")
    tags = [roots[0], *adjacent[roots[0]]]
    _, center, major, minor, xc = supports[tags[0]]
    points = [np.unique(mesh.rows["points"][patches[i][0]].reshape(-1, 3), axis=0) for i in tags]
    if any(supports[i][3:] != (minor, xc) for i in tags):
        raise ValueError("Meridians have different center stations or radii")

    def circles(params):
        a, radius = params[:2], params[2]
        return [(a, radius)] + [(b, np.linalg.norm(b - a) - radius)
                                for b in params[3:].reshape(-1, 2)]

    def residuals(params):
        return np.concatenate([
            np.hypot(p[:, 0] - xc, np.linalg.norm(p[:, 1:] - c, axis=1) - radius) - minor
            for p, (c, radius) in zip(points, circles(params))
        ])

    initial = np.r_[center, major, supports[tags[1]][1], supports[tags[2]][1]]
    fit = least_squares(residuals, initial)
    if not fit.success:
        raise ValueError("Tangent circle fit failed")
    fitted = circles(fit.x)
    reports, native = [], []
    for i, (tag, (c, radius)) in enumerate(zip(tags, fitted)):
        fixed = [None, None]
        pts = points[i]
        uv = boundary["lifted_angle"](np.arctan2(pts[:, 2] - c[1], pts[:, 1] - c[0]))
        for other in adjacent[tag]:
            delta = fitted[tags.index(other)][0] - c
            angle = np.arctan2(delta[1], delta[0])
            distances = np.abs(np.angle(np.exp(1j * (angle - np.array([uv.min(), uv.max()])))))
            side = np.argmin(distances)
            if fixed[side] is not None:
                raise ValueError("Two neighbors on the same side")
            fixed[side] = angle
        parameters = {}
        proof = boundary["chart"](mesh, patches[tag][0], c, radius, minor, xc,
                                  parameters=parameters, align=True, fixed_u=fixed)
        if not proof["admissible"]:
            return dict(admissible=False, support=tag, reason=proof["reason"])
        native.append(parameters)
        reports.append(dict(support=tag, owners=proof["owners"],
                            vertex_motion_mm=proof["vertex_motion_mm"],
                            bound_mm=proof["distance_bound_mm"]))
    if out_path is not None:
        out_path.write_text(json.dumps(dict(sectors=native), indent=2) + "\n")
    return dict(admissible=True, sectors=reports)


def qualify(rows, align=False, native_parameters=None, visit_partition=None):
    supports = []
    residual["measure"](rows, boundary["BANDS"], lambda *args: supports.append(args))
    mesh = boundary["DumpMesh"](rows)
    labels = np.full(len(rows), -1)
    for label, (ids, *_) in enumerate(supports):
        if (labels[ids] >= 0).any():
            raise ValueError("Original owners overlap")
        labels[ids] = label
    patches = []
    for ids, *geometry in supports:
        report = boundary["chart"](mesh, ids, *geometry)
        new = ids
        if not report["admissible"]:
            new, report = match(mesh, ids, *geometry, labels)
        patches.append((new, report, len(ids)))
    final = np.full(len(rows), -1)
    for tag, (ids, _, _) in enumerate(patches):
        if (final[ids] >= 0).any():
            raise ValueError("Locally unique completions collide")
        final[ids] = tag
    # Every listed triangle belongs to a whole owner and every shared edge is
    # observed from both sides with opposite winding by DumpMesh.neighbors.
    owner = rows["owner"]
    used = np.unique(owner[final >= 0])
    if np.any(np.isin(owner[final < 0], used)):
        raise ValueError("Incomplete owner")
    table = []
    incidences = {}
    for tag, (ids, report, old_count) in enumerate(patches):
        other = rim(mesh, ids)
        tags, counts = np.unique(final[other[other >= 0]], return_counts=True)
        for nbr, count in zip(tags, counts):
            if nbr >= 0:
                incidences[tag, int(nbr)] = int(count)
        table.append(dict(support=tag, triangles=len(ids), added_triangles=len(ids)-old_count,
                          owners=report["owners"], bound_mm=report["distance_bound_mm"]))
        if align:
            aligned = boundary["chart"](mesh, ids, *supports[tag][1:], trace_neighbors=True, align=True)
            table[-1]["with_aligned_rails"] = aligned
    if any(incidences.get((b, a)) != count for (a, b), count in incidences.items()):
        raise ValueError("Asymmetric interface")
    if visit_partition is not None:
        visit_partition(mesh, supports, patches, final, incidences)
    joint = common_meridians(mesh, supports, patches, incidences, native_parameters)
    return dict(joint=joint, axial_walls=axial_neighbors(mesh, supports, patches, final), sectors=table, complete_owners=len(used), triangles=int((final >= 0).sum()),
                shared_torus_interfaces=[[a, b, c] for (a, b), c in incidences.items() if a < b])


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dump", type=Path)
    parser.add_argument("--align-cylinders", action="store_true")
    parser.add_argument("--native-parameters", type=Path, help="Write the three fitted native domains locally")
    args = parser.parse_args()
    record = residual["RECORD"]
    size = args.dump.stat().st_size
    if not 0 < size <= 600000 * record.itemsize or size % record.itemsize:
        parser.error("Invalid planar dump")
    print(json.dumps(qualify(np.fromfile(args.dump, dtype=record), args.align_cylinders, args.native_parameters), indent=2))
