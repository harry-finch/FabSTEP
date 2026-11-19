"""Qualification of the known 12 mm cylindrical blank with its obliquely cut 2 mm fillet.

Input is the small mm fixture from qualify-interrupted-surfaces-freecad.py. This
laboratory oracle uses its nominal dimensions; it does not fit arbitrary STL.
"""

import math
from collections import defaultdict

from r113_mesh import read_stl


TILT = math.radians(15)
HALF_PI = math.pi / 2
VERTEX_MM = 2e-6
BUDGET_MM = 0.05
REGIONS = ("bottom", "top", "cut", "cylinder", "torus")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def cut_x(z):
    return -2 + math.tan(TILT) * (z - 6.5)


def half_span(v):
    radius = 10 + 2 * math.cos(v)
    return math.acos(-cut_x(6 + 2 * math.sin(v)) / radius)


def surface(s, v):
    radius = 10 + 2 * math.cos(v)
    angle = s * half_span(v)
    return (-radius * math.cos(angle), -radius * math.sin(angle), 6 + 2 * math.sin(v))


def unit_normal(p, q, r):
    a = tuple(y - x for x, y in zip(p, q))
    b = tuple(y - x for x, y in zip(p, r))
    cross = (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )
    length = math.hypot(*cross)
    require(length > 0, "degenerate source triangle")
    return tuple(c / length for c in cross)


def residuals(p):
    x, y, z = p
    radius = math.hypot(x, y)
    return (
        abs(z),
        abs(z - 8),
        abs(x - cut_x(z)) * math.cos(TILT),
        abs(radius - 12),
        abs(math.hypot(radius - 10, z - 6) - 2),
    )


def owner(vertices, triangle, errors):
    p, q, r = [vertices[i] for i in triangle]
    n = unit_normal(p, q, r)
    aligned = (-n[2], n[2], math.cos(TILT) * n[0] - math.sin(TILT) * n[2])
    candidates = [max(row) < VERTEX_MM for row in zip(*(errors[i] for i in triangle))]
    # Facets of a cap can have all three vertices on a circular rim. Require
    # its outward plane normal as well as the three independent distances.
    for i, cosine in enumerate(aligned):
        if candidates[i] and cosine > 1 - 1e-6:
            return REGIONS[i]
    if candidates[3] and max(p[2], q[2], r[2]) <= 6 + VERTEX_MM:
        require(
            n[0] * (p[0] + q[0] + r[0]) + n[1] * (p[1] + q[1] + r[1]) > 0,
            "cylinder winding reversed",
        )
        return "cylinder"
    require(
        candidates[4] and min(p[2], q[2], r[2]) >= 6 - VERTEX_MM,
        "unassigned triangle on known lip supports",
    )
    return "torus"


def midpoint(a, b):
    return tuple((x + y) / 2 for x, y in zip(a, b))


def triangle_bound(domain, points, depth=0):
    # Interpolate the exact patch and the original affine STL triangle on the
    # SAME parameter triangle. For barycentric weights, coordinate variances
    # are <= span²/4 and |cov| <= ds*dv/4 (also for the absolute product).
    # The weighted first-order terms cancel. The Hessian bounds below follow
    # from r=10+2cos(v), q=cut_x(z)/r, A=acos(-q), angle=s*A:
    # A<1.44, |q|<.214, |q'|<.097, |q''|<.136, |A'|<.100, |A''|<.143.
    # |Fss|<=12*A²; |Fsv|<=2*A+12*(1+A)*|A'|;
    # |Fvv|<=2+4*|A'|+12*(A'²+|A''|). Units are mm.
    ds, dv = (max(col) - min(col) for col in zip(*domain))
    drift = max(math.dist(surface(*uv), p) for uv, p in zip(domain, points))
    bound = drift + (30 * ds * ds + 12 * ds * dv + 4.5 * dv * dv) / 8 + 1e-10
    if bound <= BUDGET_MM or depth == 2:
        return bound

    # Subdivide only the proof domain. Evaluate the STL at the midpoints of
    # its original chords; projecting those midpoints would hide the error.
    def split(t):
        return (*t, *(midpoint(t[i - 1], t[i]) for i in range(3)))

    uv, xyz = split(domain), split(points)
    return max(
        triangle_bound([uv[j] for j in ids], [xyz[j] for j in ids], depth + 1)
        for ids in ((0, 3, 4), (1, 4, 5), (2, 5, 3), (3, 4, 5))
    )


def check_chart(vertices, triangles, regions, touching, edges):
    chart = {}
    for i, adjacent in touching.items():
        if "torus" not in adjacent:
            continue
        x, y, z = vertices[i]
        v = math.atan2(z - 6, math.hypot(x, y) - 10)
        if "top" in adjacent:
            v = HALF_PI
        if "cylinder" in adjacent:
            v = 0.0
        angle = math.atan2(-y, -x)
        s = math.copysign(1, angle) if "cut" in adjacent else angle / half_span(v)
        require(-1 <= s <= 1 and 0 <= v <= HALF_PI, "torus UV outside trimmed domain")
        require(math.dist(surface(s, v), vertices[i]) < VERTEX_MM, "torus vertex drift")
        chart[i] = s, v

    bound = 0.0
    for i in regions["torus"]:
        ids = triangles[i]
        uv = [chart[j] for j in ids]
        (a, b), (c, d), (e, f) = uv
        require((c - a) * (f - b) - (d - b) * (e - a) > 1e-12, "folded torus chart")
        bound = max(bound, triangle_bound(uv, [vertices[j] for j in ids]))
    require(bound <= BUDGET_MM, "torus interior exceeds budget")

    sides = defaultdict(list)
    naive_residual = 0.0
    arc_bound = 0.0
    for pair in edges.values():
        require(
            len(pair) == 2 and pair[0][1:] == pair[1][1:][::-1],
            "broken or reversed shared STL edge",
        )
        (first, a, b), (second, _, _) = pair
        if first == second or "torus" not in (first, second):
            continue
        if second == "torus":
            second, a, b = first, b, a
        (s0, v0), (s1, v1) = chart[a], chart[b]
        if second == "cut":
            require(s0 == s1 and abs(s0) == 1, "cut leaves analytic branch")
            key = "positive_s" if s0 > 0 else "negative_s"
            start, end = s0 * v0, s0 * v1
            # Along either intersection x=cut_x(z), y=±sqrt(r²-x²).
            # |x|<2.14, |x'|,|x''|<.536, |r'|,|r''|,|z''|<=2, |y|>9.76.
            # y*y'=r*r'-x*x'; y*y''=r'²+r*r''-x'²-x*x''-y'².
            # Thus |y'|<2.59, |y''|<3.72 and |(x,y,z)''|<4.3.
            drift = max(math.dist(vertices[j], surface(*chart[j])) for j in (a, b))
            arc_bound = max(arc_bound, drift + 4.3 * (v1 - v0) ** 2 / 8 + 1e-10)
            # Counterexample to interpolating a straight UV line between even
            # exact endpoints. Its midpoint is on the torus but misses the plane.
            angle = s0 * (half_span(v0) + half_span(v1)) / 2
            v = (v0 + v1) / 2
            x = -(10 + 2 * math.cos(v)) * math.cos(angle)
            naive_residual = max(
                naive_residual, abs(x - cut_x(6 + 2 * math.sin(v))) * math.cos(TILT)
            )
        else:
            require(second in ("cylinder", "top"), "unexpected neighbor of the torus")
            key = second
            expected_v = 0 if second == "cylinder" else HALF_PI
            require(v0 == v1 == expected_v, "nonhorizontal torus end")
            sign = 1 if second == "cylinder" else -1
            start, end = sign * s0, sign * s1
        require(start < end, "reversed chart boundary")
        sides[key].append((start, end, a, b))

    # Each edge has an opposite mate in the welded mesh. In the UV rectangle,
    # a once-covered positive perimeter and strictly positive triangles imply
    # multiplicity one in its interior (winding number, hence no missing area).
    extents = {
        "positive_s": (0.0, HALF_PI),
        "negative_s": (-HALF_PI, 0.0),
        "cylinder": (-1.0, 1.0),
        "top": (-1.0, 1.0),
    }
    corners = {}
    for side, limits in extents.items():
        chain = sorted(sides[side])
        require(chain and (chain[0][0], chain[-1][1]) == limits, "incomplete lip chart")
        corners[side] = (chain[0][2], chain[-1][3])
        for left, right in zip(chain, chain[1:]):
            require(
                left[1] == right[0] and left[3] == right[2],
                "gap or overlap in shared boundary chain",
            )
    order = ("cylinder", "positive_s", "top", "negative_s")
    for i, side in enumerate(order):
        require(corners[order[i - 1]][1] == corners[side][0], "split corner on the lip")
    require(
        arc_bound < BUDGET_MM and naive_residual > VERTEX_MM,
        "shared edge control failed",
    )
    return {
        "torus_patch_bidirectional_bound_mm": bound,
        "plane_torus_edges_bidirectional_bound_mm": arc_bound,
        "plane_torus_edge_segments": [
            len(sides[k]) for k in ("positive_s", "negative_s")
        ],
        "straight_uv_midpoint_plane_error_mm": naive_residual,
    }


def certify(vertices, triangles):
    errors = [residuals(p) for p in vertices]
    regions = {name: [] for name in REGIONS}
    touching = defaultdict(set)
    edges = defaultdict(list)
    for i, triangle in enumerate(triangles):
        face = owner(vertices, triangle, errors)
        regions[face].append(i)
        for j, a in enumerate(triangle):
            b = triangle[(j + 1) % 3]
            touching[a].add(face)
            edges[min(a, b), max(a, b)].append((face, a, b))
    require(all(regions.values()), "missing lip surface")
    proof = check_chart(vertices, triangles, regions, touching, edges)
    interior = [i for i, owners in touching.items() if owners == {"torus", "cut"}]
    require(interior, "missing interior lip vertex for displacement control")
    middle = min(interior, key=lambda i: (abs(vertices[i][2] - 7.0), i))
    return {**proof, "source_regions": regions}, middle


def qualify(stl_path):
    vertices, triangles = read_stl(stl_path)
    report, index = certify(vertices, triangles)
    # Move a shared vertex once in the welded table. Both neighboring regions
    # still use exactly the same vertex; topology alone cannot catch the fault.
    changed = vertices.copy()
    x, y, z = changed[index]
    changed[index] = x + 0.04, y, z
    try:
        certify(changed, triangles)
    except ValueError as error:
        report["displaced_rim_rejection"] = str(error)
    else:
        raise ValueError("displaced shared vertex accepted")
    return report
