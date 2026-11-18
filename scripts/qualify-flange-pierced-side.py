"""Prove the perforated tangent side of the flange, including every linear trim."""
import argparse
import json
from pathlib import Path
import runpy

import numpy as np

flanks = runpy.run_path(str(Path(__file__).with_name("qualify-flange-flanks.py")))
rim = flanks["rim"]
boundary = rim["boundary"]
partition = rim["partition"]
TOL = rim["TOL"]


def cross(a, b):
    return a[..., 0]*b[..., 1] - a[..., 1]*b[..., 0]


def area(polygon):
    # Keep the origin local even in translated source coordinates.
    local = polygon - polygon[0]
    return .5*float(cross(local, np.roll(local, -1, axis=0)).sum())


def check_holes(rings, length, height):
    """Strict interior, pairwise noncrossing simple cycles, without nesting."""
    if not rings or any(len(r) < 3 for r in rings) or sum(map(len, rings)) > 2048:
        raise ValueError("Unsupported number of planar trim edges")
    starts = np.concatenate(rings)
    ends = np.concatenate([np.roll(r, -1, axis=0) for r in rings])
    following = np.concatenate([np.roll(np.arange(len(r)), -1) + offset
                                for r, offset in zip(rings, np.cumsum([0] + [len(r) for r in rings[:-1]]))])
    # Refuse numerically unresolved intersections; do not repair cutouts.
    eps = 256*np.finfo(float).eps * max(1., length, height)
    guard = eps * max(1., length, height)
    if ((starts <= eps).any() or (starts >= np.array([length, height])-eps).any()
            or not np.isfinite(starts).all()):
        raise ValueError("A cutout falls outside its rectangular support")
    d = ends - starts
    if ((np.linalg.norm(d, axis=1) <= eps).any()
            or ((abs(cross(d, d[following])) <= guard) & ((d*d[following]).sum(axis=1) <= 0)).any()):
        raise ValueError("Collapsed or backtracking cutout edge")
    lo = np.minimum(starts, ends)
    hi = np.maximum(starts, ends)
    for i in range(len(starts)):
        # Only nonadjacent edges with overlapping boxes can intersect.
        others = np.arange(i + 1, len(starts))
        near = ((lo[others] <= hi[i]+eps).all(axis=1) & (lo[i] <= hi[others]+eps).all(axis=1)
                & (others != following[i]) & (following[others] != i))
        j = others[near]
        a = cross(d[i], starts[j]-starts[i])
        b = cross(d[i], ends[j]-starts[i])
        c = cross(d[j], starts[i]-starts[j])
        e = cross(d[j], ends[i]-starts[j])
        separated = (((a > guard) & (b > guard)) | ((a < -guard) & (b < -guard))
                     | ((c > guard) & (e > guard)) | ((c < -guard) & (e < -guard)))
        if not separated.all():
            raise ValueError("Overlapping or unresolved cutout edges")
    for i, ring in enumerate(rings):
        # A point of any other disjoint boundary must be outside this loop.
        for j, other in enumerate(rings):
            if i == j:
                continue
            offsets = ring-other[0]
            angles = np.arctan2(offsets[:, 1], offsets[:, 0])
            winding = np.angle(np.exp(1j*(np.roll(angles, -1)-angles))).sum()
            if abs(winding) > 1e-8:
                raise ValueError("Nested cutouts")
    return len(starts)


def canonical(ring):
    first = np.lexsort((ring[:, 1], ring[:, 0]))[0]
    return np.roll(ring, -first, axis=0)


def check_export(source, candidate):
    """Compare all directed contours as cyclic polygons, independent of ring order."""
    def normalized(rings):
        canonicalized = [canonical(np.asarray(r)) for r in rings]
        return sorted(canonicalized, key=lambda r: (r[0, 0], r[0, 1]))
    expected, observed = normalized(source), normalized(candidate)
    if (len(expected) != len(observed)
            or any(not np.array_equal(a, b) for a, b in zip(expected, observed))):
        raise ValueError("Export omits or alters a measured cutout")


def certify(mesh, ids, loops, rails, origin, direction, length, height):
    vertices, inv = np.unique(mesh.rows["points"][ids].reshape(-1, 3), axis=0, return_inverse=True)
    tri = inv.reshape(-1, 3)
    directed = np.stack([tri, np.roll(tri, -1, axis=1)], axis=2).reshape(-1, 2)
    keys = boundary["edge_key"](*directed.T, len(vertices))
    unique, counts = np.unique(keys, return_counts=True)
    shared = unique[counts == 2]
    if (len(shared) == 0 or counts.max() != 2
            or not np.isin(shared, keys[directed[:, 0] < directed[:, 1]]).all()
            or not np.isin(shared, keys[directed[:, 0] > directed[:, 1]]).all()):
        raise ValueError("Inconsistent planar triangulation")
    delta = vertices - origin
    uv = np.column_stack((delta @ direction, delta[:, 0]))
    sides = [(0, 0.), (0, length), (1, 0.), (1, height)]
    exterior = [loop for loop in loops if np.isin(rails[2], loop[0]).all()]
    if len(exterior) != 1:
        raise ValueError("Lower rail missing from the outer contour")
    outer_keys, indices, _ = exterior[0]
    labels = np.full(len(outer_keys), -1)
    for side, ((dim, value), rail) in enumerate(zip(sides, rails, strict=True)):
        use = np.isin(outer_keys, rail)
        if (use.sum() != len(rail) or np.any(labels[use] >= 0)
                or np.any(abs(uv[directed[indices[use]], dim]-value) > TOL)):
            raise ValueError("One side differs from its full adjacent rail")
        labels[use] = side
        uv[np.unique(directed[indices[use]]), dim] = value
    if (labels < 0).any():
        raise ValueError("Unmatched exterior edges")
    det, issue = boundary["rectangle_coverage"](directed[indices], labels, sides, uv, tri)
    if issue:
        raise ValueError("Invalid projected rectangle/triangles: " + issue)
    holes = []
    neighbors = []
    sign = np.sign(det[0])
    for _, ix, near in loops:
        if ix is indices:
            continue
        if np.any(near < 0):
            raise ValueError("A source cutout has unresolved neighboring triangles")
        polygon = uv[directed[ix, 0]]
        if area(polygon)*sign >= 0:
            raise ValueError("Cutout winding disagrees with the oriented source chain")
        holes.append(polygon if sign > 0 else polygon[::-1])
        neighbors.append(len(np.unique(mesh.rows["owner"][near])))
    cutout_edges = check_holes(holes, length, height)
    expected = length*height + sum(area(hole) for hole in holes)
    mesh_area = abs(det.sum())/2
    if expected <= 0 or abs(expected-mesh_area) > 512*np.finfo(float).eps*length*height:
        raise ValueError("Source area does not tile the rectangle minus its cutouts")
    reconstructed = origin + uv[:, 0, None]*direction
    reconstructed[:, 0] += uv[:, 1]
    # This interpolation is affine on every triangle. A degree-one projected
    # triangulation then gives the same global bound in both directions.
    slack = 512*np.finfo(float).eps*max(1., abs(vertices).max(), length, height)
    bound = np.linalg.norm(reconstructed-vertices, axis=1).max() + slack
    if not np.isfinite(bound) or bound > TOL:
        raise ValueError("Projection to canonical plane exceeds motion budget")
    return holes, expected, dict(triangles=len(ids), outer_edges=len(outer_keys), rails=list(map(len, rails)),
                                 cutouts=len(holes), cutout_edges=cutout_edges,
                                 edges_per_cutout=sorted(map(len, holes)), area_mm2=expected,
                                 max_projection_mm=float(bound), area_residual_mm2=abs(expected-mesh_area),
                                 neighboring_owners_per_cutout=sorted(neighbors))


def qualify(mesh, patches, labels, belt, short, check_negatives, groups=None):
    cases = []
    for tube in belt["tubes"]:
        tag, end = tube["sector"], tube["end"]
        g = belt["sectors"][tag]
        center = np.array(g["center"][1:])
        lo = g["center"][0]
        major, minor = g["major_radius_mm"], g["minor_radius_mm"]
        geo = (center, major, minor, lo)
        meridian = rim["sector_sides"](mesh, patches[tag][0], geo, g)[end]
        ids = rim["tube"](mesh, geo, meridian, labels)
        if ids is None:
            raise ValueError("Lower tangent strip lacks supported source rows")
        bottom, near = flanks["station_rail"](mesh, ids, lo)
        (owner,) = np.unique(mesh.rows["owner"][near])
        plane_ids = np.flatnonzero(mesh.rows["owner"] == owner)
        loops = list(rim["outline"](mesh, plane_ids))
        if len(loops) == 1:
            continue
        cases.append((tube, plane_ids, loops, bottom))
    if len(cases) != 1:
        raise ValueError("The rim must have one distinct perforated tangent side")
    tube, ids, loops, bottom = cases[0]
    tag, end = tube["sector"], tube["end"]
    g = belt["sectors"][tag]
    short_by_tag = {f["lower_sector"]: f for f in short}
    high = short_by_tag[tag]
    top = high["upper_torus"]
    lo, hi = g["center"][0], top["center"][0]
    center = np.array(g["center"][1:])
    major, minor = g["major_radius_mm"], g["minor_radius_mm"]
    angle = g["u_bounds_rad"][end]
    radial = np.array([np.cos(angle), np.sin(angle)])
    along = np.array(tube["extrusion_vector_world_mm"])
    length = np.linalg.norm(along)
    direction = along / length
    if not (length > TOL and direction[0] == 0 and abs(direction[1:] @ radial)*length < 1e-9):
        raise ValueError("Side direction differs from the sector tangent")
    radius = high["cylinder_radius_mm"]
    origin = np.r_[lo, center+radius*radial]
    geo = (center, major, minor, hi)
    meridian = rim["sector_sides"](mesh, patches[high["upper_sector"]][0], geo, top)[end]
    top_ids = rim["tube"](mesh, geo, meridian, labels)
    if top_ids is None:
        raise ValueError("Upper tangent strip lacks supported source rows")
    upper, near = flanks["station_rail"](mesh, top_ids, hi)
    if not np.isin(near, ids).all():
        raise ValueError("Upper tube rail does not meet the plane")
    lateral = [None, None]
    for f in short:
        other = f["lower_sector"]
        d = belt["sectors"][other]
        center = np.array(d["center"][1:])
        r = f["cylinder_radius_mm"]
        group = flanks["flank_component"](mesh, labels, patches[other][0], center, r, lo)
        _, walls, _ = flanks["cylindrical_chart"](mesh, group, center, r, d["u_bounds_rad"], lo, hi)
        for side, keys in enumerate(walls[:2]):
            if not any(np.isin(keys, k).all() for k, _, _ in loops):
                continue
            index = 0 if (other, side) == (tag, end) else 1
            if lateral[index] is not None:
                raise ValueError("Duplicated side cylinder rail")
            lateral[index] = keys
    if any(k is None for k in lateral):
        raise ValueError("Missing side cylinder rail")
    rails = [*lateral, bottom, upper]
    holes, area_mm2, report = certify(mesh, ids, loops, rails, origin, direction, length, hi-lo)
    params = dict(sector=tag, end=end, origin_mm=origin.tolist(), along_direction=direction.tolist(),
                  length_mm=float(length), height_mm=hi-lo, holes_uv_mm=[p.tolist() for p in holes],
                  area_mm2=area_mm2)
    check_export(holes, params["holes_uv_mm"])
    if check_negatives:
        reversed_ring = [holes[0][::-1], *holes[1:]]
        shifted_ring = [holes[0]+[.05, 0.], *holes[1:]]
        for bad in [holes[1:], reversed_ring, shifted_ring]:
            try:
                check_export(holes, bad)
            except ValueError:
                pass
            else:
                raise ValueError("Altered interior boundary accepted")
        missing = next(i for i, (keys, _, _) in enumerate(loops) if not np.isin(bottom, keys).all())
        cases = [("omitted_source_loop", loops[:missing] + loops[missing+1:], origin),
                 ("displaced_plane", loops, origin + np.r_[0., radial]*.01)]
        for name, observed_loops, test_origin in cases:
            try:
                certify(mesh, ids, observed_loops, rails, test_origin, direction, length, hi-lo)
            except ValueError:
                pass
            else:
                raise ValueError(f"Invalid {name} accepted by the planar proof")
        report["rejected_mutations"] = ["omitted_cutout", "reversed_cutout", "translated_cutout",
                                         *[name for name, _, _ in cases]]
    if groups is not None:
        groups["side-plane-decorated"] = ids
    return report, dict(pierced_side=params)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("dump", "belt", "flanks"):
        parser.add_argument(name, type=Path)
    parser.add_argument("--native-parameters", type=Path)
    parser.add_argument("--check-negatives", action="store_true")
    args = parser.parse_args()
    record = partition["residual"]["RECORD"]
    size = args.dump.stat().st_size
    if not 0 < size <= 600000*record.itemsize or size % record.itemsize:
        parser.error("Invalid planar dump")
    belt = json.loads(args.belt.read_text())
    short = json.loads(args.flanks.read_text())["short_flanks"]

    def visit(mesh, supports, patches, labels, incidences):
        report, params = qualify(mesh, patches, labels, belt, short, args.check_negatives)
        print(json.dumps(report, indent=2, allow_nan=False))
        if args.native_parameters:
            args.native_parameters.write_text(json.dumps(params, separators=(",", ":"), allow_nan=False)+"\n")

    partition["qualify"](np.fromfile(args.dump, dtype=record), visit_partition=visit)
