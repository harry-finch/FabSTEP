"""Offline qualification of the lower flange rim using the measured R-085 source partition."""
import argparse
import json
from pathlib import Path
import runpy
import tempfile

import numpy as np

partition = runpy.run_path(str(Path(__file__).with_name("qualify-sector-partition.py")))
boundary = partition["boundary"]
TOL = partition["TOL"]


def rim_edges(mesh, ids):
    triangles = mesh.triangles[ids]
    keys = boundary["edge_key"](triangles, np.roll(triangles, -1, axis=1), mesh.stride).ravel()
    _, first, count = np.unique(keys, return_index=True, return_counts=True)
    indices = first[count == 1]
    return keys[indices], indices, mesh.neighbors(ids, indices)


def sector_sides(mesh, ids, geometry, parameters):
    center, major, minor, xc = geometry
    keys, ix, outside = rim_edges(mesh, ids)
    tri = mesh.rows["points"][ids[ix // 3]]
    point = np.arange(len(ix))
    edges = np.stack([tri[point, ix % 3], tri[point, (ix + 1) % 3]], axis=1)
    phi = np.arctan2(edges[:, :, 2] - center[1], edges[:, :, 1] - center[0])
    sides = []
    for angle in parameters["u_bounds_rad"]:
        keep = np.abs(np.angle(np.exp(1j * (phi - angle)))).max(axis=1) * (major + minor) <= TOL
        if not keep.any() or (outside[keep] < 0).any():
            raise ValueError("Meridian rail unavailable")
        sides.append((np.sort(keys[keep]), outside[keep], angle))
    return sides


def tube(mesh, geometry, side, sector_labels):
    center, major, minor, xc = geometry
    _, seeds, angle = side
    radial = np.array([np.cos(angle), np.sin(angle)])
    points = mesh.rows["points"]
    owners = mesh.rows["owner"]
    radius = (points[:, :, 1:] - center) @ radial - major
    matches = (np.abs(np.hypot(points[:, :, 0] - xc, radius) - minor).max(axis=1) <= TOL)
    matches &= sector_labels < 0
    matches &= ~np.isin(owners, np.unique(owners[~matches]))
    if not matches[seeds].all():
        return None
    selected = np.flatnonzero(matches)
    components = boundary["components_at_seeds"](mesh, selected, seeds)
    if len(components) != 1:
        return None
    return components[0]


def prepare(mesh, supports, patches, labels, incidences):
    with tempfile.TemporaryDirectory() as tmp:
        p = Path(tmp) / "three.json"
        fit = partition["common_meridians"](mesh, supports, patches, incidences, p)
        if not fit["admissible"]:
            raise ValueError("Missing canonical torus junctions")
        joined = {row["support"]: domain for row, domain in
                  zip(fit["sectors"], json.loads(p.read_text())["sectors"], strict=True)}
    sections = {}
    rails = {}
    for tag, (ids, *geometry) in enumerate(supports):
        if geometry[-1] != 0.5:
            continue
        ids = patches[tag][0]
        fixed = [None, None]
        if tag in joined:
            fit = joined[tag]
            geometry[0] = np.asarray(fit["center"][1:])
            geometry[1] = fit["major_radius_mm"]
            observed = sector_sides(mesh, ids, geometry, fit)
            for s, (_, outside, _) in enumerate(observed):
                if (labels[outside] >= 0).all():
                    fixed[s] = fit["u_bounds_rad"][s]
        params = {}
        proof = boundary["chart"](mesh, ids, *geometry, align=True, parameters=params, fixed_u=fixed)
        if not proof["admissible"]:
            raise ValueError(f"Sector {tag}: {proof['reason']}")
        sections[tag] = (geometry, ids, fixed, params)
        for s, side in enumerate(sector_sides(mesh, ids, geometry, params)):
            rails[tag, s] = side
    return sections, rails


def discover(mesh, sections, rails, labels):
    edges = {}
    failures = []
    for key, side in rails.items():
        tag, end = key
        rail, neighbors, _ = side
        neighbor = np.unique(labels[neighbors])
        if len(neighbor) == 1 and neighbor[0] >= 0:
            other = next((k for k, v in rails.items()
                          if k[0] == neighbor[0] and np.array_equal(rail, v[0])), None)
            if other is None:
                failures.append([key, "unpaired torus seam", neighbor.tolist()])
            else:
                edges[key] = (other, "meridian", None)
            continue
        selected = tube(mesh, sections[tag][0], side, labels)
        if selected is None:
            failures.append([key, "incomplete tube", []])
            continue
        tube_edges, _, _ = rim_edges(mesh, selected)
        others = [k for k, v in rails.items() if k != key and np.isin(v[0], tube_edges).all()]
        if len(others) != 1:
            failures.append([key, "tube endpoint candidate count", others])
        else:
            edges[key] = (others[0], "tube", selected)
    return edges, failures


def common_line(a, b, sections, rails):
    # With n the physical outward meridian direction and signed guide
    # radii A,B, tangency is dot(Cb-Ca,n) = A-B.
    ga, _, _, pa = sections[a[0]]
    gb, _, _, pb = sections[b[0]]
    ca, ra, minor, xc = ga
    cb, rb, other_minor, other_xc = gb
    if minor != other_minor or xc != other_xc:
        raise ValueError("Incompatible fillet radii or stations")
    sa = np.sign(np.cos(np.mean(pa["meridian_bounds_rad"])))
    sb = np.sign(np.cos(np.mean(pb["meridian_bounds_rad"])))
    delta = cb - ca
    distance = np.linalg.norm(delta)
    direction = delta / distance
    fraction = (sa*ra - sb*rb) / distance
    if abs(fraction) >= 1:
        raise ValueError("No distinct common tangent")
    offset = np.sqrt(1 - fraction**2) * np.array([-direction[1], direction[0]])
    normals = np.array([fraction * direction + offset, fraction * direction - offset])
    def branch_error(normal):
        errors = []
        for sign, angle in [(sa, rails[a][2]), (sb, rails[b][2])]:
            sample = np.array([np.cos(angle), np.sin(angle)])
            errors.append(np.linalg.norm(sign * normal - sample))
        return max(errors)
    normal = min(normals, key=branch_error)
    na, nb = sa*normal, sb*normal
    tangent = np.array([-na[1], na[0]])
    displacement = cb + rb*nb - (ca + ra*na)
    length = displacement @ tangent
    if abs(displacement @ normal) > 1e-10 or abs(length) < TOL:
        raise ValueError("Degenerate common rail")
    return (np.arctan2(na[1],na[0]),np.arctan2(nb[1],nb[0]),length)


def verify_tube(mesh, sections, a, b, rail_a, rail_b, ids, angle, length):
    center, major, minor, xc = sections[a[0]][0]
    radial = np.array([np.cos(angle),np.sin(angle)])
    tangent = np.array([-radial[1],radial[0]])
    report = boundary["cylinder_chart"](mesh, ids, center, major, minor, xc, radial, tangent,
                                        strict_vertex=True, terminal_distance=length)
    keys, indices, _ = rim_edges(mesh, ids)
    pos = mesh.rows["points"][ids[indices//3]]
    n = np.arange(len(keys))
    endpoints = np.stack([pos[n,indices%3],pos[n,(indices+1)%3]],axis=1)
    along = (endpoints[:,:,1:] - center) @ tangent
    same_stations = all(np.array_equal(np.sort(keys[abs(along-t).max(axis=1)<=2*TOL]), rail)
                        for t,rail in [(0.,rail_a),(length,rail_b)])
    return dict(**report, complete_rails=same_stations)


def outline(mesh, ids):
    keys, ix, outside = rim_edges(mesh, ids)
    triangles = mesh.triangles[ids[ix//3]]
    row = np.arange(len(keys))
    starts = triangles[row, ix%3]
    ends = triangles[row, (ix+1)%3]
    if len(np.unique(starts)) != len(starts) or set(starts) != set(ends):
        raise ValueError("Branching or open union boundary")
    paths = {int(v):(int(w),int(i)) for i,(v,w) in enumerate(zip(starts,ends))}
    while paths:
        points = []
        start = next(iter(paths))
        vertex = start
        while vertex in paths:
            vertex,i = paths.pop(vertex)
            points.append(i)
        if vertex != start:
            raise ValueError("Open union loop")
        indices = np.asarray(points)
        yield keys[indices], ix[indices], outside[indices]


def outer_loops(mesh, ids):
    loops = []
    for _, ix, near in outline(mesh, ids):
        owners = np.unique(mesh.rows["owner"][near[near>=0]])
        edge_points = mesh.rows["points"][ids[ix//3],ix%3]
        loops.append(dict(edges=len(ix),outside_dump=int((near<0).sum()),
                          planar_neighbors=len(owners),
                          axial_range_mm=np.ptp(edge_points[:,0]),
                          axial_station_mm=np.median(edge_points[:,0])))
    return sorted(loops,key=lambda loop:loop["axial_station_mm"])


def profile_bottom_plane(mesh, belt, seams):
    loops = list(outline(mesh, belt))
    lo_keys,ix,near = min(loops,key=lambda rim: np.median(mesh.rows["points"][belt[rim[1]//3],rim[1]%3,0]))
    if (near < 0).any():
        raise ValueError("Unknown plane neighbor")
    (plane_owner,) = np.unique(mesh.rows["owner"][near])
    ids = np.flatnonzero(mesh.rows["owner"] == plane_owner)
    points = mesh.rows["points"][ids]
    origin_x = float(np.median(points[:,:,0]))
    if abs(points[:,:,0] - origin_x).max() > TOL:
        raise ValueError("Non-planar underside")
    holes, outer = [], None
    circles = []
    for keys,ix,near in outline(mesh, ids):
        yz = points[ix//3,ix%3,1:]
        if np.array_equal(np.sort(keys),np.sort(lo_keys)):
            if outer is not None:
                raise ValueError("Repeated exterior contour")
            outer = yz
            continue
        center,radius = partition["residual"]["circle"](yz)
        rho = np.linalg.norm(yz-center,axis=1)
        residual = abs(rho-radius).max()
        theta = np.arctan2(yz[:,1]-center[1], yz[:,0]-center[0])
        delta = np.angle(np.exp(1j*(np.roll(theta,-1)-theta)))
        degree = delta.sum()/(2*np.pi)
        monotone = np.all(delta>0) or np.all(delta<0)
        # Pointwise radial motion plus worst inscribed chord's sagitta, in mm.
        error = residual + radius * (1-np.cos(abs(delta).max()/2))
        circle_ok = bool(monotone and abs(abs(degree)-1)<1e-10 and residual<=TOL)
        if not circle_ok:
            raise ValueError("Inner contour is not a resolved circle")
        row = dict(edges=len(ix), planar_neighbors=int((near>=0).sum()), radius_mm=radius,
                   radial_residual_mm=residual, degree=int(round(degree)), chord_bound_mm=error)
        if seams is not None:
            uses = [(int(mesh.rows["triangle"][ids[i//3]]),int(i%3)) for i in ix]
            owners, counts = np.unique([seams[k] for k in uses],return_counts=True)
            row["analytic_seams"] = {str(k):int(n) for k,n in zip(owners,counts)}
        holes.append(row)
        circles.append((center, radius))
    if outer is None:
        raise ValueError("Plane exterior differs from the belt")
    # Check complete fitted disks lie strictly in the polygonal exterior and
    # are mutually disjoint. This concerns the measured source planar face.
    outer_area = (outer[:,0]*np.roll(outer[:,1],-1)-outer[:,1]*np.roll(outer[:,0],-1)).sum()
    edges = np.roll(outer,-1,axis=0)-outer
    for i, (c,r) in enumerate(circles):
        rel = outer-c
        angles = np.arctan2(rel[:,1],rel[:,0])
        winding = np.angle(np.exp(1j*(np.roll(angles,-1)-angles))).sum()/(2*np.pi)
        t = np.clip(-(rel*edges).sum(axis=1)/(edges*edges).sum(axis=1),0,1)
        distance = np.min(np.linalg.norm(rel+t[:,None]*edges,axis=1))
        if (abs(abs(winding)-1)>1e-10 or distance<=r+2*TOL
                or holes[i]["degree"]*outer_area>=0
                or any(np.linalg.norm(c-d)<=r+q+2*TOL for d,q in circles[:i])):
            raise ValueError("Inconsistent planar nesting")
    return dict(triangles=len(ids),outer_edges=len(outer),holes=holes,
                disjoint_circular_holes=len(holes),outer_loop_matches_belt=True)


def qualify(mesh, supports, patches, labels, incidences, native_path=None, seams=None):
    sections, rails = prepare(mesh, supports, patches, labels, incidences)
    edges, failures = discover(mesh, sections, rails, labels)
    joints = []
    constraints = {tag:list(fixed) for tag,(_,_,fixed,_) in sections.items()}
    tubes = []
    union = [ids for _,ids,_,_ in sections.values()]
    neighbors = {tag:set() for tag in sections}
    for a, (b, kind, ids) in edges.items():
        if b not in edges or edges[b][0] != a or edges[b][1] != kind:
            failures.append([a,"asymmetric joint",b])
            continue
        if a > b:
            continue
        neighbors[a[0]].add(b[0])
        neighbors[b[0]].add(a[0])
        joint = dict(a=a,b=b,kind=kind)
        if kind == "tube":
            identical = np.array_equal(ids, edges[b][2])
            joint.update(identical_triangles=identical,triangles=len(ids),
                         owners=len(np.unique(mesh.rows["owner"][ids])))
            if not identical:
                failures.append([a,"tube ownership differs at opposite ends",b])
            else:
                ua,ub,length = common_line(a,b,sections,rails)
                constraints[a[0]][a[1]] = ua
                constraints[b[0]][b[1]] = ub
                tubes.append((joint,a,b,ids,ua,length))
                union.append(ids)
        joints.append(joint)
    path = []
    tag = min(sections)
    while tag not in path:
        path.append(tag)
        others = sorted(neighbors[tag] - set(path[-2:-1]))
        if not others:
            break
        tag = others[0]
    loop = (all(len(nbr)==2 for nbr in neighbors.values()) and
            tag == path[0] and len(path) == len(sections))
    proof = {}
    nrails = {}
    native_sectors = {}
    for tag,(geometry,ids,_,_) in sections.items():
        params = {}
        result = boundary["chart"](mesh, ids, *geometry, fixed_u=constraints[tag], parameters=params)
        proof[tag] = {key:result[key] for key in ("reason","owners","distance_bound_mm","vertex_motion_mm")
                      if key in result}
        if params:
            native_sectors[tag] = params
            for i,r in enumerate(sector_sides(mesh,ids,geometry,params)):
                nrails[tag,i] = r[0]
    for joint,a,b,ids,angle,length in tubes:
        # Even when an adjacent chart fails, report tube-only geometry and
        # test its station rails independently against the source partition.
        joint["common_line"] = verify_tube(mesh, sections, a, b, nrails.get(a,rails[a][0]),
                                           nrails.get(b,rails[b][0]), ids, angle,length)
    combined = np.concatenate(union)
    if len(np.unique(combined)) != len(combined):
        failures.append("duplicate source triangle")
    owners = np.unique(mesh.rows["owner"][combined])
    if np.isin(mesh.rows["owner"],owners).sum() != len(combined):
        failures.append("incomplete planar owner")
    loops = outer_loops(mesh,combined)
    if native_path:
        if (failures or not loop or len(loops) != 2 or set(native_sectors) != set(range(7))
                or any(j["common_line"]["reason"] != "within_budget" or not j["common_line"]["complete_rails"]
                       for j, *_ in tubes) or len(tubes) != 5):
            raise ValueError("Incomplete native belt reference")
        native_tubes = [dict(sector=a[0],end=a[1],
                             extrusion_vector_world_mm=[0.,-length*np.sin(angle),length*np.cos(angle)])
                        for _,a,_,_,angle,length in tubes]
        native_path.write_text(json.dumps(dict(sectors=[native_sectors[i] for i in range(7)],
                                               tubes=native_tubes),indent=2,allow_nan=False) + "\n")
    return dict(source_cycle=loop,order=path,failures=failures,joints=joints,
                torus_proofs=proof,triangles=len(combined),owners=len(owners),
                boundary_loops=loops, bottom_plane=profile_bottom_plane(mesh,combined,seams))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dump", type=Path)
    parser.add_argument("--native-parameters", type=Path, help="Write the qualified belt to a local file")
    parser.add_argument("--analytic-seams", type=Path, help="Optional R-080 plane/analytic interface table")
    args = parser.parse_args()
    seams = None
    if args.analytic_seams:
        seams = {}
        with args.analytic_seams.open() as stream:
            for line in stream:
                tri, edge, family = line.rstrip().split("\t")
                key = (int(tri),int(edge))
                if key in seams:
                    raise ValueError("Duplicate analytic incidence")
                seams[key] = family

    record = partition["residual"]["RECORD"]
    size = args.dump.stat().st_size
    if not 0 < size <= 600000 * record.itemsize or size % record.itemsize:
        parser.error("Invalid planar dump")
    report = {}
    partition["qualify"](np.fromfile(args.dump, dtype=record),
                         visit_partition=lambda *data: report.update(qualify(*data, native_path=args.native_parameters, seams=seams)))
    print(json.dumps(report, indent=2, allow_nan=False))
