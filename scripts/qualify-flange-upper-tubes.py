"""Trace the five straight fillets on the top side from source edge incidence."""
import argparse
import json
from pathlib import Path
import runpy

import numpy as np

rim = runpy.run_path(str(Path(__file__).with_name("qualify-flange-rim.py")))
partition = rim["partition"]
boundary = rim["boundary"]
TOL = rim["TOL"]
flank_tools = runpy.run_path(str(Path(__file__).with_name("qualify-flange-flanks.py")))


def rectangular_wall(mesh, owner, top_keys, center, radius, radial, extrusion, low, high, neighbors):
    ids = np.flatnonzero(mesh.rows["owner"] == owner)
    if len(ids) != 2:
        return None
    loops = list(rim["outline"](mesh, ids))
    if len(loops) != 1 or len(loops[0][0]) != 4 or len(top_keys) != 1 or top_keys[0] not in loops[0][0]:
        raise ValueError("Side plane does not meet the upper fillet")
    keys, _, outside = loops[0]
    if (outside < 0).any() or any(np.isin(outside, ids).sum() != 1 for ids in neighbors):
        raise ValueError("Side rectangle is missing one of its four source neighbors")
    yz = center + radius * radial
    corners = np.array([[x, *(yz + t * extrusion)] for x in (low, high) for t in (0., 1.)])
    points = mesh.rows["points"][ids]
    distance = np.linalg.norm(points[:, :, None] - corners, axis=3)
    nearest = distance.argmin(axis=2)
    error = distance.min(axis=2).max()
    if error > TOL or set(nearest.ravel()) != set(range(4)):
        raise ValueError("Side plane leaves the four measured corners")
    # Opposite orientation on the common diagonal, and four exterior edges.
    edges = np.stack([nearest, np.roll(nearest, -1, axis=1)], axis=2).reshape(-1, 2)
    directed = set(map(tuple, edges))
    shared = [e for e in directed if e[::-1] in directed]
    if len(directed) != 6 or len(shared) != 2 or sum(shared[0]) != 3:
        raise ValueError("Side triangles do not span the rectangle along a diagonal")
    return float(error)


def qualify(mesh, patches, labels, belt, flanks, groups=None):
    points = mesh.rows["points"]
    owners = mesh.rows["owner"]
    upper_rails = {}
    for flank in flanks:
        upper = flank["upper_torus"]
        ids = patches[flank["upper_sector"]][0]
        geo = (np.asarray(upper["center"][1:]), upper["major_radius_mm"],
               upper["minor_radius_mm"], upper["center"][0])
        fresh = {}
        proof = boundary["chart"](mesh, ids, *geo, fixed_u=upper["u_bounds_rad"], parameters=fresh)
        if not proof["admissible"] or fresh != upper:
            raise ValueError("Upper rail parameters differ from the proven sector")
        for side, rail in enumerate(rim["sector_sides"](mesh, ids, geo, fresh)):
            upper_rails[flank["lower_sector"], side] = rail

    flanks_by_sector = {f["lower_sector"]: f for f in flanks}
    all_ids, reports, native = [], [], []
    if len(flanks_by_sector) != 4 or len(belt["tubes"]) != 5:
        raise ValueError("Incomplete upper flange reference")
    covered = set()
    rectangles = []
    for tube in belt["tubes"]:
        tag, end = tube["sector"], tube["end"]
        upper = flanks_by_sector[tag]["upper_torus"]
        rail, near, angle = upper_rails[tag, end]
        radial = np.array([np.cos(angle), np.sin(angle)])
        tangent = np.array([-radial[1], radial[0]])
        vector = np.asarray(tube["extrusion_vector_world_mm"])
        length = tangent @ vector[1:]
        if abs(vector[0]) > 1e-12 or abs(radial @ vector[1:]) > 1e-10:
            raise ValueError("Extrusion is not tangent to the upper sector")
        center = np.asarray(upper["center"][1:])
        major, minor, xc = upper["major_radius_mm"], upper["minor_radius_mm"], upper["center"][0]
        offset = points[:, :, 1:] - center
        along = offset @ tangent
        radial_dist = offset @ radial - major
        good = ((labels < 0) & (abs(np.hypot(points[:, :, 0] - xc, radial_dist) - minor).max(axis=1) <= TOL)
                & (along.min(axis=1) >= min(0., length) - TOL)
                & (along.max(axis=1) <= max(0., length) + TOL))
        good &= ~np.isin(owners, np.unique(owners[~good]))
        if not good[near].all():
            raise ValueError("Incomplete starting meridian")
        components = boundary["components_at_seeds"](mesh, np.flatnonzero(good), np.unique(near))
        if len(components) != 1:
            raise ValueError("Tube split at its source sector")
        ids = components[0]
        cert = boundary["cylinder_chart"](mesh, ids, center, major, minor, xc, radial, tangent,
                                           strict_vertex=True, terminal_distance=length)
        if cert["reason"] != "within_budget":
            raise ValueError(f"Upper tube {tag}/{end}: {cert['reason']}")
        keys, ix, neighbors = rim["rim_edges"](mesh, ids)
        pos = points[ids[ix // 3]]
        n = np.arange(len(ix))
        edges = np.stack([pos[n, ix % 3], pos[n, (ix+1) % 3]], axis=1)
        x = edges[:, :, 0]
        t = (edges[:, :, 1:] - center) @ tangent
        start = np.abs(t).max(axis=1) <= 2*TOL
        far = np.abs(t-length).max(axis=1) <= 2*TOL
        wall = np.abs(x-xc).max(axis=1) <= 2*TOL
        top = np.abs(x-(xc+minor)).max(axis=1) <= 2*TOL
        if (not np.all(start.astype(int) + far + wall + top == 1)
                or not np.array_equal(np.sort(keys[start]), rail) or (neighbors < 0).any()):
            raise ValueError("Missing source boundary")
        far_keys = np.sort(keys[far])
        joins = [k for k, (r, _, _) in upper_rails.items() if np.array_equal(r, far_keys)]
        if len(joins) > 1 or not np.all(labels[neighbors[far]] == (
                flanks_by_sector[joins[0][0]]["upper_sector"] if joins else -1)):
            raise ValueError("Ambiguous remote meridian")
        joined = [(tag,end), *joins]
        if covered.intersection(joined):
            raise ValueError("Upper meridian claimed twice")
        covered.update(joined)
        plane_owners = np.unique(owners[neighbors[wall]])
        if len(plane_owners) != 1:
            raise ValueError("Upper fillet spans several side planes")
        rectangle_bound = None
        if joins:
            lower = belt["sectors"][tag]
            v = min(upper["meridian_bounds_rad"], key=lambda v: abs(np.sin(v)))
            lower_ids = patches[tag][0]
            low_geometry = (np.asarray(lower["center"][1:]), lower["major_radius_mm"],
                            lower["minor_radius_mm"], lower["center"][0])
            low_side = rim["sector_sides"](mesh, lower_ids, low_geometry, lower)[end]
            low_tube = rim["tube"](mesh, low_geometry, low_side, labels)
            if low_tube is None:
                raise ValueError("Missing lower fillet for the straight side")
            side_neighbors = [ids, low_tube]
            for sector_tag, _ in joined:
                domain = belt["sectors"][sector_tag]
                neighbor = flank_tools["flank_component"](
                    mesh, labels, patches[sector_tag][0], np.array(domain["center"][1:]),
                    flanks_by_sector[sector_tag]["cylinder_radius_mm"], domain["center"][0])
                side_neighbors.append(neighbor)
            rectangle_bound = rectangular_wall(mesh, plane_owners[0], keys[wall], center,
                                               major + minor*np.cos(v), radial, vector[1:],
                                               lower["center"][0], xc, side_neighbors)
            if rectangle_bound is not None:
                if groups is not None:
                    groups[f"side-plane-{tag}"] = np.flatnonzero(owners == plane_owners[0])
                rectangles.append(dict(sector=tag, end=end, area_mm2=abs(length) * (xc - lower["center"][0])))
        if groups is not None:
            groups[f"upper-tube-{len(all_ids)}"] = ids
        all_ids.append(ids)
        native.append(tube)
        reports.append(dict(lower_sector=tag, end=end, remote_recognized=bool(joins),
                            triangles=len(ids),owners=len(np.unique(owners[ids])),
                            rail_edges=[int(side.sum()) for side in (start,far,wall,top)],
                            upper_plane_neighbors=len(np.unique(owners[neighbors[top]])),
                            rectangle_motion_mm=rectangle_bound,
                            **cert))
    ids = np.concatenate(all_ids)
    if (len(np.unique(ids)) != len(ids) or np.isin(owners, np.unique(owners[ids])).sum() != len(ids)
            or covered != set(upper_rails)):
        raise ValueError("Incomplete or repeated upper tube ownership")
    return dict(tubes=reports, triangles=len(ids), owners=len(np.unique(owners[ids]))), dict(upper_tubes=native, side_planes=rectangles)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("dump", "belt", "flanks"):
        parser.add_argument(name, type=Path)
    parser.add_argument("--native-parameters",type=Path)
    args = parser.parse_args()
    record = partition["residual"]["RECORD"]
    size = args.dump.stat().st_size
    if not 0 < size <= 600000 * record.itemsize or size % record.itemsize:
        parser.error("Invalid planar dump")
    belt = json.loads(args.belt.read_text())
    flanks = json.loads(args.flanks.read_text())["short_flanks"]

    def visit(mesh, supports, patches, labels, incidences):
        report, params = qualify(mesh, patches, labels, belt, flanks)
        print(json.dumps(report, indent=2, allow_nan=False))
        if args.native_parameters:
            args.native_parameters.write_text(json.dumps(params, indent=2, allow_nan=False)+"\n")

    partition["qualify"](np.fromfile(args.dump,dtype=record), visit_partition=visit)
