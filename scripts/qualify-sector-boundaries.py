"""Inspect the UV coverage and the immediate mesh neighbors of the R-085 torus candidates."""
import argparse
import json
import runpy
from pathlib import Path

import numpy as np
from scipy.optimize import minimize_scalar
from scipy.sparse import coo_matrix
from scipy.sparse.csgraph import connected_components

residual = runpy.run_path(str(Path(__file__).with_name("qualify-residual-tori.py")))
TOL = residual["TOLERANCE"]
BANDS = [(0, .5, .5), (6.5, 7, 6.5)]


def lifted_angle(value):
    """Cut the periodic coordinate in its largest unsampled gap."""
    values = np.sort(np.mod(value, 2 * np.pi))
    gaps = np.diff(np.r_[values, values[0] + 2 * np.pi])
    cut = values[gaps.argmax()] + gaps.max() / 2
    return np.mod(value - cut, 2 * np.pi) + cut


def edge_key(a, b, stride):
    return np.minimum(a, b) * stride + np.maximum(a, b)


class DumpMesh:
    def __init__(self, rows):
        self.rows = rows
        vertices, indices = np.unique(rows["points"].reshape(-1, 3), axis=0, return_inverse=True)
        self.triangles = indices.reshape(-1, 3)
        self.stride = len(vertices)
        keys = edge_key(self.triangles, np.roll(self.triangles, -1, axis=1), self.stride).ravel()
        self.order = np.argsort(keys)
        self.keys = keys[self.order]

    def neighbors(self, ids, indices):
        inside_triangle = np.repeat(ids, 3)
        start_vertex = self.triangles[inside_triangle[indices], indices % 3]
        end_vertex = self.triangles[inside_triangle[indices], (indices % 3 + 1) % 3]
        key = edge_key(start_vertex, end_vertex, self.stride)
        left = np.searchsorted(self.keys, key)
        counts = np.searchsorted(self.keys, key, side="right") - left
        if np.any((counts < 1) | (counts > 2)):
            raise ValueError("Invalid incidence in planar dump")
        other = np.full(len(indices), -1, dtype=int)
        present = counts == 2
        a = self.order[left[present]] // 3
        b = self.order[left[present] + 1] // 3
        src = inside_triangle[indices[present]]
        if np.any((a == src) == (b == src)):
            raise ValueError("Boundary has no unique interior incidence")
        edge_indices = np.where(a == src, self.order[left[present] + 1], self.order[left[present]])
        nbr_tri, local = edge_indices // 3, edge_indices % 3
        if (np.any(self.triangles[nbr_tri, local] != end_vertex[present])
                or np.any(self.triangles[nbr_tri, (local + 1) % 3] != start_vertex[present])):
            raise ValueError("Neighbor has incompatible boundary orientation")
        other[present] = nbr_tri
        return other


def rectangle_coverage(boundary, labels, sides, corrected, triangles):
    # Each side is one oriented, monotone interval between the two appropriate
    # corners. Together the four intervals provide the entire degree-one rim.
    for side, (dim, value) in enumerate(sides):
        edges = boundary[labels == side]
        varying = 1 - dim
        start, end = corrected[edges, varying].T
        if not ((end > start).all() or (end < start).all()):
            return None, "non_monotone_boundary"
        intervals = np.sort(np.stack([start, end], axis=1), axis=1)
        intervals = intervals[np.argsort(intervals[:, 0])]
        if (not np.array_equal(intervals[1:, 0], intervals[:-1, 1])
                or intervals[0, 0] != sides[2 * varying][1]
                or intervals[-1, 1] != sides[2 * varying + 1][1]):
            return None, "incomplete_side"
    for dim in range(2):
        if ((corrected[:, dim] < sides[2 * dim][1]).any()
                or (corrected[:, dim] > sides[2 * dim + 1][1]).any()):
            return None, "exterior_vertex"
    corners = corrected[triangles]
    d1, d2 = (corners[:, 1:] - corners[:, :1]).transpose(1, 0, 2)
    determinant = d1[:, 0] * d2[:, 1] - d1[:, 1] * d2[:, 0]
    guard = 128 * np.finfo(float).eps * max(1, np.max(np.abs(corrected))) ** 2
    if not ((determinant > guard).all() or (determinant < -guard).all()):
        return None, "fold_or_collapsed_triangle"
    return determinant, None


def cylinder_chart(mesh, chosen, center, major, minor, xc, radial, tangent, strict_vertex, terminal_distance=None):
    vertices, inv = np.unique(mesh.rows["points"][chosen].reshape(-1, 3), axis=0, return_inverse=True)
    triangles = inv.reshape(-1, 3)
    edges = np.stack([triangles, np.roll(triangles, -1, axis=1)], axis=2).reshape(-1, 2)
    keys = edge_key(*edges.T, len(vertices))
    _, first, count = np.unique(keys, return_index=True, return_counts=True)
    boundary = edges[first[count == 1]]
    shared = keys[first[count == 2]]
    if (count.max() > 2 or not np.isin(shared, keys[edges[:, 0] < edges[:, 1]]).all()
            or not np.isin(shared, keys[edges[:, 0] > edges[:, 1]]).all()):
        return dict(reason="incidence")
    offset = vertices[:, 1:] - center
    along = offset @ tangent
    around = lifted_angle(np.arctan2(vertices[:, 0] - xc, offset @ radial - major))
    far = np.median(along[abs(along) > max(abs(along)) - 2 * TOL])
    if terminal_distance is not None:
        far = terminal_distance
    low, high = sorted([0., float(far)])
    vlo, vhi = np.round(np.array([around.min(), around.max()]) / (np.pi / 2)) * (np.pi / 2)
    if abs(vhi - vlo - np.pi / 2) > 1e-12:
        return dict(reason="meridian_span")
    sides = [(0, low), (0, high), (1, vlo), (1, vhi)]
    uv = np.column_stack([along, around])
    d = np.stack([abs(uv[boundary, dim] - value).max(axis=1) * (1., minor)[dim]
                  for dim, value in sides])
    if not ((d <= 2 * TOL).sum(axis=0) == 1).all():
        return dict(reason="off_rectangle_or_ambiguous_edge")
    labels = np.argmin(d, axis=0)
    for side, (dim, value) in enumerate(sides):
        chain = boundary[labels == side]
        if not len(chain):
            return dict(reason="missing_side")
        uv[np.unique(chain), dim] = value
    det, reason = rectangle_coverage(boundary, labels, sides, uv, triangles)
    if reason:
        return dict(reason=reason)
    t, v = uv.T
    points = np.column_stack((xc + minor * np.sin(v),
                              center + (major + minor * np.cos(v))[:, None] * radial + t[:, None] * tangent))
    error = np.linalg.norm(points - vertices, axis=1)
    if strict_vertex and error.max() > TOL:
        return dict(reason="vertex_motion", vertex_motion_mm=float(error.max()))
    curvature = minor * np.ptp(v[triangles], axis=1) ** 2 / 8
    guard = 512 * np.finfo(float).eps * max(1., np.max(abs(vertices)), major + minor, high-low)
    bound = float((error[triangles].max(axis=1) + curvature).max() + guard)
    return dict(reason="within_budget" if bound <= .05 else "budget", jacobian_min_abs=float(abs(det).min()),
                signed_distance_mm=far, vertex_motion_mm=float(error.max()), distance_bound_mm=bound)


def components_at_seeds(mesh, selected, seeds):
    """Return selected triangles joined by edges to the provided seed rows."""
    tri = mesh.triangles[selected]
    keys = edge_key(tri, np.roll(tri, -1, axis=1), mesh.stride).ravel()
    order = np.argsort(keys)
    paired = np.diff(keys[order]) == 0
    left, right = order[:-1][paired] // 3, order[1:][paired] // 3
    graph = coo_matrix((np.ones(len(left), dtype=bool), (left, right)),
                       shape=(len(selected), len(selected)))
    _, component = connected_components(graph, directed=False)
    roots = np.unique(component[np.searchsorted(selected, seeds)])
    return [selected[component == root] for root in roots]


def tangent_strip(mesh, ids, seeds, angle, center, major, minor, axial_center, strict_vertex):
    """Trace whole exterior planar owners compatible with the tangent cylinder."""
    faces = mesh.rows["points"]
    radial = np.array([np.cos(angle), np.sin(angle)])
    tangent = np.array([-np.sin(angle), np.cos(angle)])
    radial_offset = (faces[:, :, 1:] - center) @ radial - major
    circle_error = abs(np.hypot(faces[:, :, 0] - axial_center, radial_offset) - minor)
    matches = circle_error.max(axis=1) <= TOL
    matches[ids] = False
    rejected_owners = np.unique(mesh.rows["owner"][~matches])
    matches &= ~np.isin(mesh.rows["owner"], rejected_owners)
    selected = np.flatnonzero(matches)
    kept_seeds = np.isin(seeds, selected)
    result = dict(seed_edges=len(seeds), complete_seed_edges=int(kept_seeds.sum()))
    if not kept_seeds.any():
        return result
    components = components_at_seeds(mesh, selected, seeds[kept_seeds])
    result["seed_components"] = len(components)
    details = []
    for chosen in components:
        verts, inv = np.unique(faces[chosen].reshape(-1, 3), axis=0, return_inverse=True)
        local = inv.reshape(-1, 3)
        edges = np.stack([local, np.roll(local, -1, axis=1)], axis=2).reshape(-1, 2)
        _, first, count = np.unique(edge_key(*edges.T, len(verts)), return_index=True,
                                    return_counts=True)
        rim = edges[first[count == 1]]
        around = lifted_angle(np.arctan2(verts[:, 0] - axial_center,
                                         (verts[:, 1:] - center) @ radial - major))
        along = (verts[:, 1:] - center) @ tangent
        coords = np.column_stack([along, minor * around])
        ends = np.array([coords.min(axis=0), coords.max(axis=0)])
        residual = np.stack([abs(coords[rim] - value).max(axis=1) for value in ends])
        residual = residual.transpose(0, 2, 1).reshape(4, -1)
        on_side = residual <= 2 * TOL
        rect = (on_side.sum(axis=0) == 1).all()
        # Check that the four supporting isocurves actually carry contiguous
        # side chains between their common corner vertices.
        chains = []
        for side in range(4):
            chain = rim[on_side[side]]
            nodes, degrees = np.unique(chain, return_counts=True)
            ends_of_chain = nodes[degrees == 1]
            varying = 1 - side % 2
            delta = coords[chain[:, 1], varying] - coords[chain[:, 0], varying]
            valid = (len(chain) > 0 and len(ends_of_chain) == 2 and degrees.max() <= 2
                     and np.unique(chain[:, 0]).size == len(chain)
                     and np.unique(chain[:, 1]).size == len(chain)
                     and ((delta > 0).all() or (delta < 0).all()))
            if valid:
                # Directed monotonicity precludes loops, leaving one path.
                actual = np.sort(coords[ends_of_chain, varying])
                valid = bool((abs(actual - ends[:, varying]) <= 2 * TOL).all())
            chains.append(bool(valid))
        # A single unrelated face on the extrapolated cylinder can spoil its
        # apparent outer limit. Count complete owners past the best populated
        # end station of the rim to document this possible source of leakage.
        terminal = {}
        if not rect:
            positions = abs(along)
            rim_position = positions[np.unique(rim)]
            outer = rim_position[rim_position > np.max(rim_position) / 2]
            bins, counts = np.unique(np.round(outer, 4), return_counts=True)
            if len(bins):
                station = float(np.median(outer[abs(outer - bins[counts.argmax()]) < 0.000051]))
                past = positions[local].max(axis=1) > station + 2 * TOL
                terminal = dict(populated_end_distance_mm=station, triangles_past_end=int(sum(past)),
                                owners_touching_past_end=len(np.unique(mesh.rows["owner"][chosen[past]])))
                excluded = np.unique(mesh.rows["owner"][chosen[past]])
                keep = ~np.isin(mesh.rows["owner"][chosen], excluded)
                if keep.any():
                    terminal["candidate_station_cut"] = dict(
                        retained_triangles=int(keep.sum()), removed_triangles=int(sum(~keep)),
                        removed_complete_owners=len(excluded),
                        removed_analysis_triangle_ids=mesh.rows["triangle"][chosen[~keep]].tolist(),
                        chart=cylinder_chart(mesh, chosen[keep], center, major, minor, axial_center, radial, tangent, strict_vertex))
        details.append(dict(**terminal, chart=cylinder_chart(mesh, chosen, center, major, minor,
                                                            axial_center, radial, tangent, strict_vertex), owners=len(np.unique(mesh.rows["owner"][chosen])), triangles=len(chosen),
                            axial_extent_mm=float(np.ptp(along)),
                            meridian_deg=float(np.rad2deg(np.ptp(around))),
                            boundary_edges=len(rim),
                            irregular_edges=int(np.sum(on_side.sum(axis=0) != 1)),
                            greatest_boundary_miss_mm=float(residual.min(axis=0).max()),
                            four_complete_sides=bool(rect and all(chains))))
    result["components"] = details
    return result


def fit_boundary_angle(vertices, uv, boundary, labels, sides, side, neighbors, mesh,
                       center, major, minor, xc):
    """Minimize tube residual inside the admissible interval of shared seam angles."""
    angle = sides[side][1]
    nodes = np.unique(boundary[labels == side])
    incident = neighbors[labels == side]
    if (incident < 0).any():
        return angle
    v = uv[:, 1].copy()
    for i in (2, 3):
        v[np.unique(boundary[labels == i])] = sides[i][1]
    model_rho = major + minor * np.cos(v[nodes])
    rho = np.linalg.norm(vertices[nodes, 1:] - center, axis=1)
    dx = vertices[nodes, 0] - xc - minor * np.sin(v[nodes])
    # Squared distance at angular shift a from the source azimuth is the
    # squared meridian residual plus 4*rho*model_rho*sin(a/2)^2.
    remaining = TOL**2 - dx**2 - (rho - model_rho)**2
    if (remaining <= 0).any():
        return angle
    span = np.minimum(TOL / (major + minor),
                      2 * np.arcsin(np.sqrt(remaining / (4 * rho * model_rho))))
    shift = uv[nodes, 0] - angle
    low = max(-TOL / (major + minor), float(max(shift - span)))
    high = min(TOL / (major + minor), float(min(shift + span)))
    if high <= low:
        return angle
    points = mesh.rows["points"][incident]
    def objective(delta):
        radial = np.array([np.cos(angle + delta), np.sin(angle + delta)])
        radius = (points[:, :, 1:] - center) @ radial - major
        return np.abs(np.hypot(points[:, :, 0] - xc, radius) - minor).max()
    optimum = minimize_scalar(objective, bounds=(low, high), method="bounded",
                              options={"xatol": 1e-15})
    return angle + min((optimum.x, np.clip(0., low, high)), key=objective)


def chart(mesh, ids, center, major, minor, axial_center, trace_neighbors=False, parameters=None, align=False, fixed_u=None):
    faces = mesh.rows["points"][ids]
    vertices, inv = np.unique(faces.reshape(-1, 3), axis=0, return_inverse=True)
    triangles = inv.reshape(-1, 3)
    dy, dz = (vertices[:, 1:] - center).T
    rho = np.hypot(dy, dz)
    torus_residual = float(np.max(abs(np.hypot(vertices[:, 0] - axial_center, rho - major) - minor)))
    u = lifted_angle(np.arctan2(dz, dy))
    v = lifted_angle(np.arctan2(vertices[:, 0] - axial_center, rho - major))
    directed = np.stack([triangles, np.roll(triangles, -1, axis=1)], axis=2).reshape(-1, 2)
    keys = edge_key(*directed.T, len(vertices))
    _, first, counts = np.unique(keys, return_index=True, return_counts=True)
    boundary_indices = first[counts == 1]
    boundary = directed[boundary_indices]
    # Verify cancellation as a directed chain. A common Jacobian sign and a
    # once-covered rectangle boundary then imply degree one in its interior.
    positive_keys = np.sort(keys[directed[:, 0] < directed[:, 1]])
    negative_keys = np.sort(keys[directed[:, 0] > directed[:, 1]])
    shared = keys[first[counts == 2]]
    cancels = (counts.max() == 2 and np.isin(shared, positive_keys).all()
               and np.isin(shared, negative_keys).all())
    result = dict(triangles=len(ids), owners=len(np.unique(mesh.rows["owner"][ids])),
                  boundary_edges=len(boundary), support_residual_mm=torus_residual)
    def reject(reason):
        return dict(**result, admissible=False, reason=reason)

    if not (np.isfinite(torus_residual) and major > minor > 0 and torus_residual <= TOL):
        return reject("unsupported_witness")
    if not len(boundary):
        return reject("missing_boundary")
    if not cancels:
        return reject("interior_incidence")
    uv = np.array([u, v]).T
    rim = uv[np.unique(boundary)]
    scales = np.array([major + minor, minor])
    if np.any(np.ptp(uv, axis=0) >= np.pi):
        return reject("coordinate_span")

    # The end faces determine the physical quarter of the meridian. For the
    # two azimuth cuts, estimate the common angle from clustered rim vertices.
    sides = []
    for dim, scale in enumerate(scales):
        for extreme in (rim[:, dim].min(), rim[:, dim].max()):
            if dim == 1:
                value = np.round(extreme / (np.pi / 2)) * (np.pi / 2)
            else:
                cluster = abs(rim[:, dim] - extreme) * scale < 2 * TOL
                value = np.median(rim[cluster, dim])
            sides.append((dim, value))
    if abs(sides[3][1] - sides[2][1] - np.pi / 2) > 1e-12:
        return reject("meridian_span")
    def rim_labels():
        distance = np.array([abs(uv[boundary, dim] - value).max(axis=1) * scales[dim]
                             for dim, value in sides])
        result["boundary_miss_mm"] = float(distance.min(axis=0).max())
        membership = distance <= TOL
        if not (membership.sum(axis=0) == 1).all():
            return None
        return np.argmax(membership, axis=0)

    labels = rim_labels()
    if labels is None:
        return reject("off_rectangle_or_ambiguous_edge")
    corrected = uv.copy()
    neighbors = mesh.neighbors(ids, boundary_indices)
    if fixed_u is not None and len(fixed_u) != 2:
        raise ValueError("fixed_u must supply minimum and maximum meridian angles in radians")
    if align or fixed_u is not None:
        initial = tuple(s[1] for s in sides[:2])
        for side in (0, 1):
            angle = initial[side]
            if fixed_u is not None and fixed_u[side] is not None:
                requested = float(fixed_u[side])
                if not np.isfinite(requested):
                    return reject("non_finite_meridian")
                angle = requested + 2 * np.pi * round((angle - requested) / (2 * np.pi))
            elif align:
                angle = fit_boundary_angle(vertices, uv, boundary, labels, sides, side, neighbors,
                                           mesh, center, major, minor, axial_center)
            sides[side] = (0, angle)
        shifts = [(sides[i][1] - initial[i]) * (major + minor) for i in (0, 1)]
        result["signed_angular_shifts_mm"] = shifts
        if np.max(np.abs(shifts)) > TOL:
            return reject("meridian_adjustment")
        labels = rim_labels()
        if labels is None:
            return reject("off_rectangle_or_ambiguous_edge")
    summaries = []
    for side, (dim, value) in enumerate(sides):
        select = labels == side
        edges = boundary[select]
        if not edges.size:
            return reject("missing_side")
        other = neighbors[select]
        pts = mesh.rows["points"][other[other >= 0]]
        summary = dict(edges=len(edges), outside_dump=int(sum(other < 0)),
                       adjacent_plane_owners=len(np.unique(mesh.rows["owner"][other[other >= 0]])))
        if dim == 0:
            # Tangent extrusion of the meridian circle along the azimuth
            # tangent. These are observations on the first neighbor row only.
            relative = pts[:, :, 1:] - center
            radial = relative @ np.array([np.cos(value), np.sin(value)])
            err = abs(np.hypot(pts[:, :, 0] - axial_center, radial - major) - minor)
            summary["tangent_tube_neighbors"] = int(np.sum(err.max(axis=1) <= TOL))
        else:
            radius = major + minor * np.cos(value)
            error_plane = abs(pts[:, :, 0] - axial_center - minor * np.sin(value))
            error_cylinder = abs(np.linalg.norm(pts[:, :, 1:] - center, axis=2) - radius)
            summary["axial_plane_neighbors"] = int(np.sum(error_plane.max(axis=1) <= TOL))
            summary["coaxial_cylinder_neighbors"] = int(np.sum(error_cylinder.max(axis=1) <= TOL))
        summaries.append(summary)
        corrected[np.unique(edges), dim] = value
    if trace_neighbors and (align or all(summary["tangent_tube_neighbors"] == summary["edges"]
                               for summary in summaries[:2])):
        for side in (0, 1):
            if summaries[side]["tangent_tube_neighbors"] != summaries[side]["edges"]:
                continue
            summaries[side]["tangent_strip"] = tangent_strip(
                mesh, ids, neighbors[labels == side], sides[side][1],
                center, major, minor, axial_center, strict_vertex=align or fixed_u is not None)
    result["neighbors_u_min_u_max_v_min_v_max"] = summaries
    determinant, reason = rectangle_coverage(boundary, labels, sides, corrected, triangles)
    if reason:
        return reject(reason)
    result["jacobian_min_abs"] = float(abs(determinant).min())
    u, v = corrected.T
    rho = major + minor * np.cos(v)
    points = np.column_stack((axial_center + minor * np.sin(v),
                              center[0] + rho * np.cos(u), center[1] + rho * np.sin(u)))
    residuals = np.linalg.norm(points - vertices, axis=1)
    if (align or fixed_u is not None) and residuals.max() > TOL:
        return reject("vertex_motion")
    du, dv = np.ptp(corrected[triangles], axis=1).T
    # Vector-valued Taylor remainder with barycentric variances <= span²/4
    # and Cauchy-Schwarz for the weighted mean of |delta_u * delta_v|.
    interpolation = ((major + minor) * du**2 + minor * dv**2) / 8 + minor * du * dv / 4
    slack = 512 * np.finfo(float).eps * max(1, np.max(abs(vertices)), major + minor)
    bound = float(np.max(residuals[triangles].max(axis=1) + interpolation) + slack)
    result.update(azimuth_deg=float(np.rad2deg(sides[1][1] - sides[0][1])),
                  vertex_motion_mm=float(residuals.max()), distance_bound_mm=bound)
    if parameters is not None and bound <= .05:
        u0, u1 = (s[1] for s in sides[:2])
        parameters.update(center=[axial_center, *center], axis=[1., 0., 0.],
                          start_direction=[0., np.cos(u0), np.sin(u0)], u_bounds_rad=[u0, u1],
                          sweep_rad=u1-u0, meridian_bounds_rad=[s[1] for s in sides[2:]],
                          major_radius_mm=major, minor_radius_mm=minor)
        extrusions = []
        for side, summary in enumerate(summaries[:2]):
            strips = summary.get("tangent_strip", {}).get("components", [])
            if len(strips) != 1 or strips[0]["chart"]["reason"] != "within_budget":
                break
            angle = sides[side][1]
            along = strips[0]["chart"]["signed_distance_mm"]
            direction = np.array([0., -np.sin(angle), np.cos(angle)])
            extrusions.append(dict(angle_rad=angle, signed_distance_mm=along,
                                   direction_plus_u_world=direction.tolist(),
                                   extrusion_vector_world_mm=(along * direction).tolist()))
        if len(extrusions) == 2:
            parameters["extrusions"] = extrusions
    return dict(**result, admissible=bound <= .05, reason="within_budget" if bound <= .05 else "budget")


def chart_from_triangles(triangles, sector_indices, center, major, minor, axial_center):
    """Qualify a supplied patch in an X-axial frame, with one owner per triangle."""
    rows = np.empty(len(triangles), dtype=residual["RECORD"])
    rows["owner"] = np.arange(len(triangles))
    rows["points"] = triangles
    return chart(DumpMesh(rows), np.asarray(sector_indices, dtype=int), np.asarray(center),
                 major, minor, axial_center)


def examine_station_cuts(mesh, sectors, bands):
    """Look for other measured supports at whole owners removed by a populated end rail."""
    rows = mesh.rows
    order = np.argsort(rows["triangle"])
    triangles = rows["points"]
    for sector in sectors:
        xc = sector["band_center_mm"]
        neighbors = sector.get("neighbors_u_min_u_max_v_min_v_max", [])
        for seam in neighbors[:2]:
            for strip in seam.get("tangent_strip", {}).get("components", []):
                cut = strip.get("candidate_station_cut")
                if not cut:
                    continue
                analysis_ids = cut["removed_analysis_triangle_ids"]
                seeds = order[np.searchsorted(rows["triangle"][order], analysis_ids)]
                candidates = []
                for other_xc, center, major, minor in bands:
                    if other_xc == xc:
                        continue
                    def error(points):
                        rho = np.linalg.norm(points[:, :, 1:] - center, axis=2)
                        return abs(np.hypot(points[:, :, 0] - xc, rho - major) - minor)
                    seed_error = float(error(triangles[seeds]).max())
                    if seed_error > TOL:
                        continue
                    start, end, _ = next(band for band in BANDS if xc == band[2])
                    axial = triangles[:, :, 0]
                    matches = ((error(triangles).max(axis=1) <= TOL)
                               & (axial.min(axis=1) >= start-TOL) & (axial.max(axis=1) <= end+TOL)
                               & (np.ptp(axial, axis=1) > TOL))
                    matches &= ~np.isin(rows["owner"], np.unique(rows["owner"][~matches]))
                    if not matches[seeds].all():
                        continue
                    for ids in components_at_seeds(mesh, np.flatnonzero(matches), seeds):
                        proof = chart(mesh, ids, center, major, minor, xc)
                        candidates.append(dict(triangles=len(ids), owners=proof["owners"],
                                               seed_residual_mm=seed_error, rectangle=proof["reason"]))
                cut["other_band_torus_supports"] = candidates


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dump", type=Path)
    parser.add_argument("--align-cylinders", action="store_true",
                        help="also test a bounded angular fit to the first row across each meridian")
    parser.add_argument("--native-parameters", type=Path,
                        help="write the unique sector with two fully covered neighboring tubes to a local file")
    args = parser.parse_args()
    size = args.dump.stat().st_size
    record = residual["RECORD"]
    if not 0 < size <= 600000 * record.itemsize or size % record.itemsize:
        parser.error("Invalid planar dump")
    rows = np.fromfile(args.dump, dtype=record)
    mesh = DumpMesh(rows)
    sectors = []
    native_cases = []
    supports = []
    def visit(ids, center, major, minor, xc):
        parameters = {}
        supports.append((xc, center, major, minor))
        sector = chart(mesh, ids, center, major, minor, xc, trace_neighbors=True, parameters=parameters)
        if args.align_cylinders and sector["admissible"]:
            sector["aligned"] = chart(mesh, ids, center, major, minor, xc, trace_neighbors=True, align=True)
        if "extrusions" in parameters:
            native_cases.append(parameters)
        sectors.append(dict(band_center_mm=xc, **sector))
    residual["measure"](rows, BANDS, visit)
    examine_station_cuts(mesh, sectors, supports)
    if args.native_parameters:
        if len(native_cases) != 1:
            parser.error("expected one complete native reference")
        args.native_parameters.write_text(json.dumps(native_cases[0], indent=2, allow_nan=False) + "\n")
    print(json.dumps(sectors, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
