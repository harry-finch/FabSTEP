"""Offline R-085 proof of the X-axial cylinder patches adjoining the lower rim.

The upper trim of a nonrectangular patch is a measured path in cylinder UV.
Its neighboring analytic faces still need to be established before CAD use.
"""
import argparse
import json
from pathlib import Path
import runpy
import tempfile

import numpy as np

rim = runpy.run_path(str(Path(__file__).with_name("qualify-flange-rim.py")))
partition = rim["partition"]
boundary = partition["boundary"]
TOL = partition["TOL"]


def station_rail(mesh, ids, station):
    keys, indices, near = rim["rim_edges"](mesh, ids)
    x = mesh.rows["points"][ids[indices // 3], :, 0]
    row = np.arange(len(indices))
    at = ((abs(x[row, indices % 3] - station) <= TOL)
          & (abs(x[row, (indices + 1) % 3] - station) <= TOL))
    if not at.any() or (near[at] < 0).any():
        raise ValueError("Unresolved station rail")
    return np.sort(keys[at]), near[at]


def flank_component(mesh, labels, lower_ids, center, radius, station):
    _, seeds = station_rail(mesh, lower_ids, station)
    points = mesh.rows["points"]
    owners = mesh.rows["owner"]
    radial = np.linalg.norm(points[:, :, 1:] - center, axis=2)
    fits = ((abs(radial - radius).max(axis=1) <= TOL)
            & (points[:, :, 0].min(axis=1) >= station - TOL)
            & (np.ptp(points[:, :, 0], axis=1) > TOL) & (labels < 0))
    fits &= ~np.isin(owners, np.unique(owners[~fits]))
    if not fits[seeds].all():
        raise ValueError("Part of the lower rail lacks a cylindrical neighbor")
    connected = boundary["components_at_seeds"](mesh, np.flatnonzero(fits), np.unique(seeds))
    if len(connected) != 1:
        raise ValueError("Separated sidewall components on the lower rail")
    ids = connected[0]
    if np.isin(owners, np.unique(owners[ids])).sum() != len(ids):
        raise ValueError("Incomplete sidewall owner")
    return ids


def check_trim(uv, loop_edges, labels, sides):
    # The four chains are traversed in cyclic order. A monotone upper arc
    # may contain vertical jumps, provided they never double back at one u.
    # A simple boundary and consistently oriented interior triangles give a
    # degree-one map onto the measured UV polygon.
    if np.count_nonzero(labels != np.roll(labels, 1)) != 4:
        raise ValueError("Disconnected side of the cylinder trim")
    for side, (dim, value) in enumerate(sides):
        edges = loop_edges[labels == side]
        if not len(edges):
            raise ValueError("Missing trim side")
        uv[np.unique(edges), dim] = value
    for side, (dim, _) in enumerate(sides):
        delta = np.diff(uv[loop_edges[labels == side], 1 - dim], axis=1).ravel()
        if not ((delta > 0).all() or (delta < 0).all()):
            raise ValueError("Backtracking straight side")
    # Rotate at the beginning of the upper chain, including paths crossing
    # the original loop's list boundary.
    first = np.flatnonzero((labels == 3) & (np.roll(labels, 1) != 3))
    if len(first) != 1:
        raise ValueError("Upper trim must follow one source chain")
    edges = np.roll(loop_edges, -first[0], axis=0)[:np.count_nonzero(labels == 3)]
    upper = uv[np.r_[edges[:, 0], edges[-1, 1]]]
    lo, hi = sides[0][1], sides[1][1]
    if set(upper[[0, -1], 0]) != {lo, hi} or (upper[:, 1] <= sides[2][1]).any():
        raise ValueError("Upper trim fails to span the azimuth above the lower rail")
    delta = np.diff(upper, axis=0)
    du, dx = delta.T
    direction = np.sign(upper[-1, 0] - upper[0, 0])
    if (direction * du < 0).any() or (du[[0, -1]] == 0).any():
        raise ValueError("Upper trim doubles back or overlaps an end rail")
    vertical = du == 0
    if (dx[vertical] == 0).any() or ((vertical[1:] & vertical[:-1]) & (dx[1:] * dx[:-1] <= 0)).any():
        raise ValueError("Overlapping edges in the upper trim")
    return int(vertical.sum())


def cylindrical_chart(mesh, ids, center, radius, u_limits, xlo, xhi=None):
    vertices, inv = np.unique(mesh.rows["points"][ids].reshape(-1, 3), axis=0, return_inverse=True)
    tri = inv.reshape(-1, 3)
    directed = np.stack([tri, np.roll(tri, -1, axis=1)], axis=2).reshape(-1, 2)
    edge_keys = boundary["edge_key"](*directed.T, len(vertices))
    keys, counts = np.unique(edge_keys, return_counts=True)
    shared = keys[counts == 2]
    if (counts.max() != 2 or not np.isin(shared, edge_keys[directed[:, 0] < directed[:, 1]]).all()
            or not np.isin(shared, edge_keys[directed[:, 0] > directed[:, 1]]).all()):
        raise ValueError("Interior edges fail directed cancellation")
    loops = list(rim["outline"](mesh, ids))
    if len(loops) != 1:
        raise ValueError("Cylindrical patch must have one boundary")
    keys, indices, near = loops[0]
    ring = directed[indices]
    offset = vertices[:, 1:] - center
    low, high = u_limits
    if not (radius > 0 and 0 < high-low < np.pi):
        raise ValueError("Unsupported cylindrical sweep")
    u = (low + high)/2 + np.angle(np.exp(1j*(np.arctan2(offset[:, 1], offset[:, 0]) - (low+high)/2)))
    uv = np.column_stack([u, vertices[:, 0]])
    sides = [(0, low), (0, high), (1, xlo)]
    if xhi is not None:
        sides.append((1, xhi))
    membership = np.array([abs(uv[ring, dim] - value).max(axis=1) * (radius, 1.)[dim] <= TOL
                           for dim, value in sides])
    if (membership.sum(axis=0) > 1).any():
        raise ValueError("Ambiguous trim side")
    labels = np.where(membership.any(axis=0), membership.argmax(axis=0), 3)
    vertical = 0
    if xhi is None:
        vertical = check_trim(uv, ring, labels, sides)
    else:
        if not membership.any(axis=0).all() or len(np.unique(labels)) != 4:
            raise ValueError("Rectangle fails its station or meridian limits")
        for side, (dim, value) in enumerate(sides):
            uv[np.unique(ring[labels == side]), dim] = value
        _, reason = boundary["rectangle_coverage"](ring, labels, sides, uv, tri)
        if reason:
            raise ValueError(reason)
    a, b = (uv[tri[:, 1:]] - uv[tri[:, :1]]).transpose(1, 0, 2)
    det = a[:, 0] * b[:, 1] - a[:, 1] * b[:, 0]
    guard = 128 * np.finfo(float).eps * max(1., abs(uv).max())**2
    if not ((det > guard).all() or (det < -guard).all()):
        raise ValueError("Fold or collapsed triangle in cylinder coordinates")
    u, x = uv.T
    projected = np.column_stack([x, center[0] + radius*np.cos(u), center[1] + radius*np.sin(u)])
    motion = np.linalg.norm(projected - vertices, axis=1)
    # The second derivative in u has norm radius, the axial and mixed
    # derivatives vanish. The same UV correspondence covers both directions.
    slack = 512*np.finfo(float).eps * max(1., abs(vertices).max(), radius)
    distance = float((motion[tri].max(axis=1) + radius*np.ptp(u[tri], axis=1)**2/8).max() + slack)
    if motion.max() > TOL or distance > .05:
        raise ValueError("Source-to-cylinder distance budget exceeded")
    sides_keys = [np.sort(keys[labels == i]) for i in range(4)]
    report = dict(triangles=len(ids), owners=len(np.unique(mesh.rows["owner"][ids])),
                  rail_edges=[len(k) for k in sides_keys], distance_bound_mm=distance,
                  vertex_motion_mm=float(motion.max()), upper_vertical_edges=vertical,
                  upper_station_range_mm=[float(f(x[np.unique(ring[labels == 3])])) for f in (min, max)])
    return report, sides_keys, near[labels == 3]


def side_junctions(mesh, belt, labels, flanks):
    """Match each cylinder end to another cylinder or a whole planar owner."""
    owners = mesh.rows["owner"]
    terminal = {(i, e): keys for i, (_, ends) in enumerate(flanks) for e, keys in enumerate(ends)}
    matched = set()
    direct = []
    for a, keys in terminal.items():
        others = [b for b, other in terminal.items() if b != a and np.array_equal(keys, other)]
        if len(others) > 1:
            raise ValueError("Ambiguous common sidewall generatrix")
        if others and a < others[0]:
            b = others[0]
            # rim_edges checks opposite directed incidence in the dump.
            all_keys, _, near = rim["rim_edges"](mesh, flanks[a[0]][0])
            neighbors = near[np.isin(all_keys, keys)]
            if not np.isin(neighbors, flanks[b[0]][0]).all():
                raise ValueError("Cylinder end lacks neighboring cylinder")
            direct.append(dict(ends=[a, b], source_edges=len(keys)))
            matched.update((a, b))
    flat = []
    plane_owners = set()
    for tube in belt["tubes"]:
        tag, end = tube["sector"], tube["end"]
        lower = belt["sectors"][tag]
        center = np.array(lower["center"][1:])
        xlo = lower["center"][0]
        major, minor = lower["major_radius_mm"], lower["minor_radius_mm"]
        angle = lower["u_bounds_rad"][end]
        normal = np.array([np.cos(angle), np.sin(angle)])
        geometry = (center, major, minor, xlo)
        lower_ids = np.flatnonzero(labels == tag)
        meridian = rim["sector_sides"](mesh, lower_ids, geometry, lower)[end]
        tube_ids = rim["tube"](mesh, geometry, meridian, labels)
        if tube_ids is None:
            raise ValueError("Lower cylindrical fillet has no complete patch")
        bottom, near = station_rail(mesh, tube_ids, xlo)
        (owner,) = np.unique(owners[near])
        if owner in plane_owners:
            raise ValueError("Straight fillets claim the same side plane")
        plane_owners.add(owner)
        ids = np.flatnonzero(owners == owner)
        points = mesh.rows["points"][ids]
        v = min(lower["meridian_bounds_rad"], key=lambda v: abs(np.sin(v)))
        radius = major + minor*np.cos(v)
        deviation = abs((points[:, :, 1:] - center) @ normal - radius).max()
        if not np.isfinite(deviation) or deviation > TOL:
            raise ValueError("Neighbor is outside the canonical tangent plane")
        loops = list(rim["outline"](mesh, ids))
        candidates = [(k, ix) for k, ix, _ in loops if np.isin(bottom, k).all()]
        if len(candidates) != 1:
            raise ValueError("Lower fillet rail split between plane contours")
        outer, indices = candidates[0]
        ends = [a for a, keys in terminal.items() if np.isin(keys, outer).all()]
        if (len(ends) != 2 or (tag, end) not in ends or matched.intersection(ends)
                or np.intersect1d(terminal[ends[0]], terminal[ends[1]]).size):
            raise ValueError("Plane must share two distinct cylinder ends")
        matched.update(ends)
        remaining = np.setdiff1d(outer, np.concatenate([bottom, *(terminal[a] for a in ends)]))
        if not len(remaining):
            raise ValueError("No upper boundary on the side plane")
        upper = indices[np.isin(outer, remaining)]
        upper_tri = points[upper // 3]
        n = np.arange(len(upper))
        x = np.r_[upper_tri[n, upper % 3, 0], upper_tri[n, (upper+1) % 3, 0]]
        rectangle = len(loops) == 1 and len(bottom) == len(remaining) == 1 and len(outer) == 4
        flat.append(dict(cylinder_ends=ends, triangles=len(ids), lower_straight_edges=len(bottom),
                         cylinder_end_edges=[len(terminal[a]) for a in ends], outline_edges=len(outer),
                         other_loop_edges=sorted(len(k) for k, _, _ in loops if k is not outer),
                         other_exterior_edges=len(remaining), upper_x_span_mm=float(np.ptp(x)),
                         four_straight_edges=rectangle, normal_residual_mm=float(deviation)))
    if matched != set(terminal):
        raise ValueError("A cylinder end has no resolved side ownership")
    return dict(direct_cylinder_junctions=direct, tangent_plane_neighbors=flat)


def qualify(mesh, supports, patches, labels, incidences, native_out=None, negative=False, belt=None, groups=None):
    if belt is None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "belt.json"
            rim["qualify"](mesh, supports, patches, labels, incidences, native_path=path)
            belt = json.loads(path.read_text())
    report, upper_sectors, flanks = [], [], []
    seen = np.zeros(len(labels), dtype=bool)
    refused = []
    for tag, domain in enumerate(belt["sectors"]):
        low_ids = patches[tag][0]
        xlo, *yz = domain["center"]
        center = np.array(yz)
        major, minor = domain["major_radius_mm"], domain["minor_radius_mm"]
        meridian = min(domain["meridian_bounds_rad"], key=lambda v:abs(np.sin(v)))
        if abs(np.sin(meridian)) > 1e-10:
            raise ValueError("Expected a meridian tangent to the cylinder")
        radius = major + minor*np.cos(meridian)
        limits = domain["u_bounds_rad"]
        ids = flank_component(mesh, labels, low_ids, center, radius, xlo)
        if seen[ids].any():
            raise ValueError("A sidewall owner was already used")
        seen[ids] = True
        neighbors = partition["rim"](mesh, ids)
        adjacent = set(labels[neighbors[neighbors >= 0]]) - {-1, tag}
        if len(adjacent) > 1:
            raise ValueError("Ambiguous connection to upper sectors")
        upper_tag = next(iter(adjacent), None)
        xhi = None if upper_tag is None else supports[upper_tag][-1]
        checked, rails, near = cylindrical_chart(mesh, ids, center, radius, limits, xlo, xhi)
        flanks.append((ids, rails[:2]))
        low_keys, _ = station_rail(mesh, low_ids, xlo)
        if not np.array_equal(rails[2], low_keys):
            raise ValueError("Incomplete lower-sector rail")
        checked["lower_sector"] = tag
        if upper_tag is not None:
            hi_ids = patches[upper_tag][0]
            upper_keys, _ = station_rail(mesh, hi_ids, xhi)
            if (xhi <= xlo or np.any(near < 0) or not np.all(labels[near] == upper_tag)
                    or not np.array_equal(rails[3], upper_keys)):
                raise ValueError("Incomplete upper-sector rail")
            params = {}
            high = boundary["chart"](mesh, hi_ids, center, major, minor, xhi, parameters=params, fixed_u=limits)
            if (not high["admissible"] or not any(abs(np.sin(v)) < 1e-10
                    and abs(major + minor*np.cos(v) - radius) < 1e-10 for v in params["meridian_bounds_rad"])):
                raise ValueError("Upper torus incompatible with the same canonical cylinder")
            if groups is not None:
                groups[f"short-flank-{tag}"] = ids
                groups[f"torus-{upper_tag}"] = hi_ids
            checked.update(upper_sector=int(upper_tag), upper_torus_bound_mm=high["distance_bound_mm"])
            upper_sectors.append(dict(lower_sector=tag, upper_sector=int(upper_tag),
                                      cylinder_radius_mm=radius, upper_torus=params))
        else:
            checked.update(upper_neighbors_known=int((near >= 0).sum()),
                           upper_neighbors_owners=len(np.unique(mesh.rows["owner"][near[near >= 0]])))
        if negative:
            # Replay plausible geometric and topological errors on real triangles.
            rows = mesh.rows[ids].copy()
            rows["points"][0] = rows["points"][0, ::-1].copy()
            reversed_triangle = boundary["DumpMesh"](rows)
            # Long patches include a plateau at the upper fillet station.
            wrong_top = xhi + .01 if xhi is not None else max(support[-1] for support in supports)
            cases = [("radius", mesh, ids, radius + .01, limits, xhi),
                     ("wrong_upper_plane", mesh, ids, radius, limits, wrong_top),
                     ("azimuth", mesh, ids, radius, np.array(limits) + .01, xhi),
                     ("flipped_triangle", reversed_triangle, np.arange(len(ids)), radius, limits, xhi)]
            for case, source, group, test_radius, azimuth, top in cases:
                try:
                    cylindrical_chart(source, group, center, test_radius, azimuth, xlo, top)
                except ValueError:
                    refused.append([tag, case])
                else:
                    raise ValueError(f"Invalid {case} accepted on sector {tag}")
        report.append(checked)
    sides = side_junctions(mesh, belt, labels, flanks)
    if native_out:
        native_out.write_text(json.dumps(dict(short_flanks=upper_sectors), indent=2, allow_nan=False) + "\n")
    return dict(flanks=report, **sides, refused_counterexamples=refused)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dump", type=Path)
    parser.add_argument("--native-parameters", type=Path, help="Write the four certified rectangular references locally")
    parser.add_argument("--check-negatives", action="store_true")
    args = parser.parse_args()
    record = partition["residual"]["RECORD"]
    size = args.dump.stat().st_size
    if not 0 < size <= 600000 * record.itemsize or size % record.itemsize:
        parser.error("Invalid planar dump")
    report = {}
    partition["qualify"](np.fromfile(args.dump, dtype=record), visit_partition=lambda *data:
                         report.update(qualify(*data, native_out=args.native_parameters, negative=args.check_negatives)))
    print(json.dumps(report, indent=2, allow_nan=False))
