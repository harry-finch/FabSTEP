"""R-113 laboratory certificate for the cross_bored_shaft() millimetre fixture.

qualify() uses the independent fixture dimensions. qualify_fit() checks the
analytic trimmed surfaces proposed from STL measurements, including their
intersection. Neither needs CAD face labels or a reference mesh.
"""

import argparse
import json
import math
from collections import defaultdict
from pathlib import Path
from typing import NamedTuple

from r113_mesh import read_stl


class Dimensions(NamedTuple):
    radius: float
    height: float
    bore_radius: float
    bore_y: float
    bore_z: float


NOMINAL = Dimensions(10.0, 16.0, 2.4, 3.2, 7.25)
FIT_MM, BUDGET_MM = 1e-5, 0.05
ROUND_MM = 1e-10


def require(condition, message):
    if not condition:
        raise ValueError(message)


def turn(angle):
    return math.remainder(angle, math.tau)


def shaft_angle(p):
    return math.atan2(p[1], p[0])


def bore_angle(p, d):
    return math.atan2(p[2] - d.bore_z, p[1] - d.bore_y)


def oriented_area2(uv):
    (a, b), (c, d), (e, f) = uv
    return (c - a) * (f - b) - (e - a) * (d - b)


def source_area2(points):
    a, b, c = points
    u, v = ([q - p for q, p in zip(other, a)] for other in (b, c))
    return math.hypot(
        u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]
    )


def cylinder_patch(points, owner, d, area2):
    if owner == "shaft":
        radius = d.radius
        angles = [shaft_angle(p) for p in points]
        heights = [p[2] for p in points]
        radial = [math.hypot(p[0], p[1]) for p in points]
        orientation = 1
    else:
        radius = d.bore_radius
        angles = [bore_angle(p, d) for p in points]
        heights = [p[0] for p in points]
        radial = [math.hypot(p[1] - d.bore_y, p[2] - d.bore_z) for p in points]
        orientation = -1  # The bore wall faces the void.
    residual = max(abs(r - radius) for r in radial)
    angles = [turn(a - angles[0]) for a in angles]
    span = max(angles) - min(angles)
    chart = list(zip([radius * a for a in angles], heights))
    if (
        residual > FIT_MM
        or span >= math.pi
        or orientation * oriented_area2(chart) <= max(1e-10, area2 / 2)
    ):
        return None
    # Linear interpolation on an angular interval of width h has radial-vector
    # error <= R*h^2/8 (second derivative norm R, angle variance <= h^2/4).
    return residual + radius * span**2 / 8 + ROUND_MM


def assign(points, d):
    candidates = {}
    area2 = source_area2(points)
    for key, z, orientation in [("bottom", 0.0, -1), ("top", d.height, 1)]:
        residual = max(abs(p[2] - z) for p in points)
        if residual <= FIT_MM and orientation * oriented_area2(
            [p[:2] for p in points]
        ) > max(1e-10, area2 / 2):
            candidates[key] = residual + ROUND_MM
    for key in ("shaft", "bore"):
        bound = cylinder_patch(points, key, d, area2)
        if bound is not None and bound < BUDGET_MM:
            candidates[key] = bound
    require(
        len(candidates) == 1,
        f"expected one oriented support, found {sorted(candidates)}",
    )
    return next(iter(candidates.items()))


def cycle(edges):
    successor = dict(edges)
    require(
        len(successor) == len(edges) >= 3 and set(successor) == set(successor.values()),
        "branched or open rim",
    )
    ids = [min(successor)]
    while len(ids) < len(edges):
        ids.append(successor[ids[-1]])
    require(
        len(set(ids)) == len(ids) and successor[ids[-1]] == ids[0],
        "multiple rim cycles",
    )
    return ids


def rim_bound(name, points, d):
    cap = name in ("top", "bottom")
    sign = 1 if name in ("right", "bottom") else -1
    # The shaft face's lower boundary advances in phi. On the right its
    # circular hole is clockwise in (R*phi,z), hence decreasing bore angle.
    direction = sign if cap else -sign
    angles = [shaft_angle(p) if cap else bore_angle(p, d) for p in points]
    gaps = [turn(q - a) for a, q in zip(angles, angles[1:] + angles[:1])]
    require(
        all(direction * gap > 1e-10 for gap in gaps)
        and abs(math.fsum(gaps) - direction * math.tau) < 1e-9,
        f"incorrect angular coverage of {name}",
    )
    gap2 = max(abs(gap) for gap in gaps) ** 2 / 8
    if cap:
        z = d.height if name == "top" else 0.0
        radial = max(abs(math.hypot(p[0], p[1]) - d.radius) for p in points)
        axial = max(abs(p[2] - z) for p in points)
        return {"shaft": axial + ROUND_MM, name: radial + d.radius * gap2 + ROUND_MM}

    # The two analytic intersection branches are parameterized by bore angle t:
    # y = Y+r*cos(t), z = Z+r*sin(t), x = +/-sqrt(R²-y²).
    # Compare their curves with the PL rims separately in both isometric
    # cylinder charts and in space. Each interpolation error is bounded by
    # max |curve''(t)| * max gap²/8 plus the measured endpoint discrepancy.
    ymax = abs(d.bore_y) + d.bore_radius
    xmin = math.sqrt(d.radius**2 - ymax**2)
    dx2 = d.bore_radius * ymax / xmin + d.bore_radius**2 * d.radius**2 / xmin**3
    # s=R*asin(y/R) (up to sign and offset). Combine the first derivative
    # with (y'',z'') of norm r, then add the s''(y)*y'² term.
    ds_z2 = (
        d.bore_radius * d.radius / xmin + d.bore_radius**2 * d.radius * ymax / xmin**3
    )
    drift = {"shaft": 0.0, "bore": 0.0, "curve_3d": 0.0}
    for p, a in zip(points, angles):
        y = d.bore_y + d.bore_radius * math.cos(a)
        q = (
            sign * math.sqrt(d.radius**2 - y**2),
            y,
            d.bore_z + d.bore_radius * math.sin(a),
        )
        ds = d.radius * turn(shaft_angle(p) - shaft_angle(q))
        drift["shaft"] = max(drift["shaft"], math.hypot(ds, p[2] - q[2]))
        drift["bore"] = max(drift["bore"], abs(p[0] - q[0]))
        drift["curve_3d"] = max(drift["curve_3d"], math.dist(p, q))
    require(drift["curve_3d"] < FIT_MM, f"{name} vertices outside the common curve")
    return {
        "shaft": drift["shaft"] + ds_z2 * gap2 + ROUND_MM,
        "bore": drift["bore"] + dx2 * gap2 + ROUND_MM,
        "curve_3d": drift["curve_3d"]
        + math.hypot(dx2, d.bore_radius) * gap2
        + ROUND_MM,
    }


def certificate(vertices, triangles, dimensions=NOMINAL):
    r, height, bore, offset, level = dimensions
    require(
        all(math.isfinite(v) and abs(v) < 32 for v in dimensions)
        and bore > 2 * BUDGET_MM
        and abs(offset) + bore + 2 * BUDGET_MM < r
        and bore + 2 * BUDGET_MM < level < height - bore - 2 * BUDGET_MM,
        "bore branches or cap clearances outside certified domain",
    )
    require(
        all(all(math.isfinite(v) and abs(v) < 32 for v in p) for p in vertices),
        "outside shaft fixture coordinates",
    )
    groups = {key: [] for key in ("bottom", "top", "shaft", "bore")}
    interior = dict.fromkeys(groups, 0.0)
    edges = defaultdict(list)
    for index, tri in enumerate(triangles):
        owner, bound = assign([vertices[i] for i in tri], dimensions)
        interior[owner] = max(interior[owner], bound)
        groups[owner].append(index)
        for a, b in zip(tri, (tri[1], tri[2], tri[0])):
            edges[min(a, b), max(a, b)].append((owner, a, b))
    require(all(groups.values()), "missing source region")
    rims = {k: [] for k in ("bottom", "top", "right", "left")}
    for uses in edges.values():
        require(len(uses) == 2, "an edge does not have two incident triangles")
        (left, a, b), (right, c, d) = uses
        require((a, b) == (d, c), "an edge has incompatible orientations")
        if left == right:
            continue
        if right == "shaft":
            left, right, a, b = right, left, c, d
        require(left == "shaft", "unexpected pair of neighboring surfaces")
        if right == "bore":
            require(vertices[a][0] * vertices[b][0] > 0, "rim jumps between mouths")
            right = "right" if vertices[a][0] > 0 else "left"
        rims[right].append((a, b))
    rim_ids = {name: cycle(edges) for name, edges in rims.items()}
    rim_counts = {name: len(ids) for name, ids in rim_ids.items()}
    rim_bounds = {
        name: rim_bound(name, [vertices[i] for i in ids], dimensions)
        for name, ids in rim_ids.items()
    }
    # On each chart the oriented planar triangles have a positive multiplicity
    # (negative consistently for the bore and base). Their interior edges cancel.
    # The four once-winding rims above are the entire boundary. Straight-line
    # homotopy from each PL rim to its analytic rim therefore preserves the
    # coverage degree away from the bounded rim strips. In each swept strip
    # both domains have boundary points within the chart-distance bound. Mapping
    # back from unwrapped cylindrical arc length and height is nonexpansive.
    bounds = {
        owner: bound + max(r.get(owner, 0.0) for r in rim_bounds.values())
        for owner, bound in interior.items()
    }
    require(max(bounds.values()) <= BUDGET_MM, "bidirectional fidelity exceeds 0.05 mm")
    return {
        "source_regions": groups,
        "rim_edges": rim_counts,
        "mouth_chord_bounds_mm": {
            k: rim_bounds[k]["curve_3d"] for k in ("right", "left")
        },
        "bidirectional_bounds_mm": bounds,
    }, rim_ids["right"][0]


def qualify_fit(vertices, triangles, fit):
    """Certify the measured supports in the fitted shaft's right-handed frame."""
    from r113_shaft_fit import cross, dot, sub

    z, x = fit["shaft_axis"], fit["bore_axis"]
    require(
        all(
            len(v) == 3 and all(math.isfinite(n) for n in v)
            for v in (z, x, fit["shaft_origin"], fit["bore_origin"])
        ),
        "invalid measured axes or origins",
    )
    require(
        abs(dot(x, x) - 1) < 2e-15
        and abs(dot(z, z) - 1) < 2e-15
        and abs(dot(z, x)) < 2e-15,
        "fitted frame is not orthonormal",
    )
    frame = (x, cross(z, x), z)
    origin = fit["shaft_origin"]

    def local(p):
        require(
            all(math.isfinite(n) and abs(n) < 1024 for n in (*p, *origin)),
            "world coordinates outside laboratory rounding margin",
        )
        delta = sub(p, origin)
        return tuple(dot(delta, direction) for direction in frame)

    _, offset, level = local(fit["bore_origin"])
    dims = Dimensions(
        fit["shaft_radius_mm"], fit["height_mm"], fit["bore_radius_mm"], offset, level
    )
    proof, _ = certificate([local(p) for p in vertices], triangles, dims)
    require(
        fit["source_regions"] == proof["source_regions"],
        "fitted triangle owners differ",
    )
    return proof


def qualify_fitted_pose(vertices, triangles, fit):
    """Replay an oblique f32 STL pose and a deformed bore vertex."""
    from struct import Struct
    from r113_shaft_fit import cross, dot, fit_shaft, radial, shift, sub, unit

    axis = unit((1, 3, 2))
    sine, cosine = math.sin(0.89), math.cos(0.89)
    translation = (15.0, -7.0, 21.0)
    binary = Struct("<3f")

    def rotate(p):
        return tuple(
            cosine * a + sine * b + (1 - cosine) * dot(axis, p) * c
            for a, b, c in zip(p, cross(axis, p), axis)
        )

    def move(p):
        return tuple(r + t for r, t in zip(rotate(p), translation))

    moved = [binary.unpack(binary.pack(*move(p))) for p in vertices]
    candidate = fit_shaft(moved, triangles)
    proof = qualify_fit(moved, triangles, candidate)
    require(
        candidate["source_regions"] == fit["source_regions"],
        "pose changed source owners",
    )
    for key in ("shaft_origin", "bore_origin"):
        require(
            math.dist(candidate[key], move(fit[key])) < FIT_MM,
            "pose changed axis origin",
        )
    for key in ("shaft_axis", "bore_axis"):
        require(
            math.hypot(*cross(candidate[key], rotate(fit[key]))) < 1e-6,
            "pose changed axis direction",
        )
    for key in ("height_mm", "bore_radius_mm", "shaft_radius_mm"):
        require(abs(fit[key] - candidate[key]) < FIT_MM, "pose changed a dimension")

    damaged = list(vertices)
    i = triangles[fit["source_regions"]["bore"][0]][0]
    offset = unit(radial(sub(damaged[i], fit["bore_origin"]), fit["bore_axis"]))
    damaged[i] = shift(damaged[i], offset, 0.03)
    try:
        fit_shaft(damaged, triangles)
    except ValueError as error:
        refusal = str(error)
    else:
        raise ValueError("fitted incompatible bore vertex")
    return {
        "max_vertex_residual_mm": candidate["max_vertex_residual_mm"],
        "bidirectional_bounds_mm": proof["bidirectional_bounds_mm"],
        "deformed_vertex_refusal": refusal,
    }


def qualify(stl_path):
    vertices, triangles = read_stl(stl_path)
    proof, vertex = certificate(vertices, triangles)
    damaged = list(vertices)
    x, y, z = damaged[vertex]
    damaged[vertex] = (x, y, z + 0.03)
    try:
        certificate(damaged, triangles)
    except ValueError as error:
        proof["deformed_mouth_refusal"] = str(error)
    else:
        raise ValueError("damaged rim accepted")
    return proof


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("stl", type=Path)
    print(json.dumps(qualify(parser.parse_args().stl), indent=2))
