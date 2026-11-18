"""Planar source-to-analytic contour correspondence for the R-085 private laboratory."""
import numpy as np

VERTEX_TOL = 1e-5
TRANSFER_TOL = 5e-8
DISTANCE_LIMIT = .05


def cross(a, b):
    return a[..., 0] * b[..., 1] - a[..., 1] * b[..., 0]


def circle_points(center, radius, angle):
    return center + radius * np.column_stack((np.cos(angle), np.sin(angle)))


def rail(points, sector, extrusion, end):
    """Match one contiguous chain, including endpoints, to its whole arc or segment."""
    center = np.asarray(sector["center"][1:])
    radius = sector["major_radius_mm"]
    u0, u1 = sector["u_bounds_rad"]
    if extrusion is None:
        da = np.arctan2(points[:, 1] - center[1], points[:, 0] - center[0]) - u0
        angles = np.angle(np.exp(1j * da)) + u0
        if angles[-1] > angles[0]:
            angles[0], angles[-1] = u0, u1
        else:
            angles[0], angles[-1] = u1, u0
        steps = np.diff(angles)
        if not ((steps > 0).all() or (steps < 0).all()) or np.max(abs(steps)) >= np.pi:
            raise ValueError("Folded exterior arc")
        target = circle_points(center, radius, angles)
        twice_area = cross(center, target[-1] - target[0]) + radius**2 * steps.sum()
        sag = 2 * radius * np.sin(steps / 4)**2
    else:
        start = circle_points(center, radius, np.array([sector["u_bounds_rad"][end]]))[0]
        span = np.asarray(extrusion)[1:]
        t = (points - start) @ span / (span @ span)
        t[0], t[-1] = (0., 1.) if t[-1] > t[0] else (1., 0.)
        if not ((np.diff(t) > 0).all() or (np.diff(t) < 0).all()):
            raise ValueError("Folded exterior segment")
        target = start + t[:, None] * span
        twice_area = cross(target[0], target[-1])
        sag = 0.
    motion = np.linalg.norm(target - points, axis=1)
    if not np.isfinite(motion).all() or max(motion) > VERTEX_TOL:
        raise ValueError("Exterior rail deviates from its source vertices")
    bound = (np.maximum(motion[:-1], motion[1:]) + sag).max()
    return target[0], target[-1], twice_area, bound


def exterior(points, neighbors, face_indices, sectors, tubes):
    tags = face_indices[neighbors]
    if (tags < 0).any() or set(tags) != set(range(len(sectors) + len(tubes))):
        raise ValueError("Exterior does not meet every belt face")
    starts = np.flatnonzero(tags != np.roll(tags, 1))
    if len(starts) != len(set(tags)):
        raise ValueError("Disconnected rail on a belt face")
    pieces = []
    for begin, stop in zip(starts, np.r_[starts[1:], starts[0] + len(tags)]):
        chain = points[np.arange(begin, stop + 1) % len(tags)]
        tag = tags[begin]
        if tag < len(sectors):
            args = sectors[tag], None, None
        else:
            tube = tubes[tag - len(sectors)]
            args = sectors[tube["sector"]], tube["extrusion_vector_world_mm"], tube["end"]
        pieces.append(rail(chain, *args))
    gap = max(np.linalg.norm(p[1] - q[0]) for p, q in zip(pieces, pieces[1:] + pieces[:1]))
    if gap > TRANSFER_TOL:
        raise ValueError("Exterior analytic rail endpoints disagree")
    return sum(p[2] for p in pieces) / 2, max(p[3] for p in pieces)


def cone_hole(cone, owner, station):
    base, end = cone["base_ring"], cone["exit_ring"]
    near, far = np.asarray(base["center"]), np.asarray(end["center"])
    radii = [base["radius_mm"], end["radius_mm"]]
    if (cone["base_neighbor"] != {"kind": "plane", "index": int(owner)}
            or not cone["inner"] or min(radii) <= 0
            or not np.isfinite([*near, *far, *radii]).all()
            or abs(near[0] - station) > TRANSFER_TOL or far[0] <= station
            or not np.array_equal(near[1:], far[1:])
            or any(not np.array_equal(r["axis"], [1., 0., 0.]) for r in (base, end))):
        raise ValueError("Cone rings incompatible with the selected underside")
    return dict(cone_index=cone["cone_index"], center_yz_mm=near[1:].tolist(),
                radius_mm=radii[0], far_radius_mm=radii[1], far_station_mm=far[0])
