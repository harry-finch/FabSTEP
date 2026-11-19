"""Laboratory fit for a round shaft with two circular caps and a transverse bore.

Input is one oriented, welded, small STL from r113_mesh.read_stl. All geometric
parameters and triangle owners are determined from its coordinates.
"""

import math
from collections import defaultdict

FIT = 1e-5  # mm
BUDGET = 0.05  # mm


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sub(p, q):
    return tuple(a - b for a, b in zip(p, q))


def shift(p, d, k):
    return tuple(a + k * b for a, b in zip(p, d))


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def cross(a, b):
    return (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )


def unit(v):
    length = math.hypot(*v)
    require(length > 1e-12, "degenerate direction")
    return tuple(a / length for a in v)


def mean(points):
    return tuple(math.fsum(row) / len(points) for row in zip(*points))


def radial(v, axis):
    return shift(v, axis, -dot(v, axis))


def circle(points, axis):
    # The farthest point from the first and the farthest from their chord
    # span a stable triangle. Every sample independently verifies its circle.
    p = points[0]
    samples = [radial(sub(v, p), axis) for v in points]
    a = max(samples, key=lambda v: dot(v, v))
    b = max(samples, key=lambda v: abs(dot(cross(a, v), axis)))
    denominator = 2 * dot(cross(a, b), axis)
    require(
        abs(denominator) > 1e-6 and abs(denominator) > dot(a, a) * 0.1,
        "poorly constrained circular support",
    )
    center = shift(
        shift(p, cross(b, axis), dot(a, a) / denominator),
        cross(axis, a),
        dot(b, b) / denominator,
    )
    radii = [math.hypot(*radial(sub(q, center), axis)) for q in points]
    radius = math.fsum(radii) / len(radii)
    require(
        max(abs(r - radius) for r in radii) <= FIT, "circular fit exceeds tolerance"
    )
    return center, radius


def cycles(edges):
    following = dict(edges)
    require(
        len(following) == len(edges) and set(following) == set(following.values()),
        "open or branched region boundary",
    )
    pending = set(following)
    loops = []
    while pending:
        start = p = min(pending)
        loop = []
        while p in pending:
            pending.remove(p)
            loop.append(p)
            p = following[p]
        require(p == start and len(loop) >= 8, "incomplete circular witness")
        loops.append(loop)
    return loops


def single_turn(points, center, axis):
    vectors = [radial(sub(p, center), axis) for p in points]
    increments = [
        math.atan2(dot(cross(a, b), axis), dot(a, b))
        for a, b in zip(vectors, vectors[1:] + vectors[:1])
    ]
    signed = math.fsum(increments)
    require(
        abs(abs(signed) - math.tau) < 1e-8
        and all(
            delta * signed > 0 and abs(delta) < math.pi / 2 for delta in increments
        ),
        "ring does not wind once monotonically",
    )
    return signed


def projected_cylinder(points, axis, center, radius, sense):
    vectors = [sub(p, center) for p in points]
    radial_vectors = [radial(p, axis) for p in vectors]
    error = max(abs(math.hypot(*p) - radius) for p in radial_vectors)
    if error > FIT:
        return None
    ref = radial_vectors[0]
    angles = [math.atan2(dot(cross(ref, p), axis), dot(ref, p)) for p in radial_vectors]
    heights = [dot(p, axis) for p in vectors]
    winding = angles[1] * (heights[2] - heights[0]) - angles[2] * (
        heights[1] - heights[0]
    )
    span = max(angles) - min(angles)
    if (
        sense * radius * winding <= 1e-10
        or span >= math.pi
        or error + radius * span**2 / 8 > BUDGET
    ):
        return None
    return error


def fit_shaft(vertices, triangles):
    """Return both axes, radii, cap separation and disjoint source triangle sets."""
    require(4 <= len(triangles) <= 10_000, "expected a small closed witness")
    by_edge = defaultdict(list)
    normals, area_vectors = [], []
    for i, (a, b, c) in enumerate(triangles):
        vector = cross(sub(vertices[b], vertices[a]), sub(vertices[c], vertices[a]))
        normals.append(unit(vector))
        area_vectors.append(vector)
        for x, y in ((a, b), (b, c), (c, a)):
            by_edge[min(x, y), max(x, y)].append((i, x, y))
    adjacency = [[] for _ in triangles]
    for uses in by_edge.values():
        require(len(uses) == 2, "open or non-manifold source")
        (i, a, b), (j, c, d) = uses
        require((a, b) == (d, c), "inconsistent winding")
        if dot(normals[i], normals[j]) > 1 - 5e-7:
            adjacency[i].append(j)
            adjacency[j].append(i)

    def region_boundary(ids):
        edges = []
        for i in ids:
            a, b, c = triangles[i]
            for x, y in ((a, b), (b, c), (c, a)):
                if any(
                    other not in ids for other, _, _ in by_edge[min(x, y), max(x, y)]
                ):
                    edges.append((x, y))
        return edges

    caps = []
    visited = set()
    for i in range(len(triangles)):
        if i in visited:
            continue
        ids, stack = {i}, [i]
        visited.add(i)
        while stack:
            for j in adjacency[stack.pop()]:
                if j not in visited:
                    visited.add(j)
                    ids.add(j)
                    stack.append(j)
        if len(ids) < 6:
            continue
        try:
            normal = unit(mean([area_vectors[t] for t in sorted(ids)]))
            loops = cycles(region_boundary(ids))
            require(len(loops) == 1, "cap has more than one rim")
            ring = [vertices[v] for v in loops[0]]
            center, radius = circle(ring, normal)
            residual = max(
                abs(dot(sub(vertices[v], center), normal))
                for t in ids
                for v in triangles[t]
            )
            require(
                residual <= FIT and single_turn(ring, center, normal) > 0,
                "cap plane or orientation unproved",
            )
        except ValueError:
            continue
        caps.append((ids, normal, center, radius, residual))
    require(len(caps) == 2, "expected exactly two circular caps")
    bottom, top = caps
    center = bottom[2]
    axis = unit(sub(top[2], center))
    height = math.dist(top[2], center)
    require(
        dot(bottom[1], axis) < -1 + 1e-10
        and dot(top[1], axis) > 1 - 1e-10
        and abs(top[3] - bottom[3]) <= FIT,
        "caps do not establish one shaft",
    )
    radius = (bottom[3] + top[3]) / 2
    owners = {
        "bottom": sorted(bottom[0]),
        "top": sorted(top[0]),
        "shaft": [],
        "bore": [],
    }
    residuals = [
        max(
            abs(dot(sub(vertices[v], center), axis) - level)
            for i in ids
            for v in triangles[i]
        )
        for ids, level in ((owners["bottom"], 0.0), (owners["top"], height))
    ]
    for i, t in enumerate(triangles):
        if i in bottom[0] or i in top[0]:
            continue
        error = projected_cylinder([vertices[v] for v in t], axis, center, radius, 1)
        if error is None:
            owners["bore"].append(i)
        else:
            owners["shaft"].append(i)
            residuals.append(error)
    wall = set(owners["bore"])
    require(wall and owners["shaft"], "missing interrupted wall or bore")
    boundary = region_boundary(wall)
    shaft_ids = set(owners["shaft"])
    require(
        all(
            any(i in shaft_ids for i, _, _ in by_edge[min(a, b), max(a, b)])
            for a, b in boundary
        ),
        "bore crosses an end cap",
    )
    mouths = cycles(boundary)
    require(len(mouths) == 2, "expected two bore mouths")

    # On a ruled band one generator direction lies in each facet plane. Use
    # two well-separated normals as a proposal, then check every bore vertex.
    n0 = normals[min(wall)]
    n1 = min((normals[i] for i in sorted(wall)), key=lambda n: abs(dot(n, n0)))
    require(abs(dot(n0, n1)) < 0.5, "indeterminate bore direction")
    inferred = unit(cross(n0, n1))
    require(abs(dot(inferred, axis)) < 1e-5, "bore is not transverse to the shaft")
    bore_axis = unit(radial(inferred, axis))
    samples = [vertices[i] for i in sorted({v for t in wall for v in triangles[t]})]
    bore_center, bore_radius = circle(samples, bore_axis)
    middle = shift(center, axis, height / 2)
    bore_center = shift(
        bore_center, bore_axis, dot(sub(middle, bore_center), bore_axis)
    )
    signs = [
        single_turn([vertices[v] for v in ids], bore_center, bore_axis)
        for ids in mouths
    ]
    require(signs[0] * signs[1] < 0, "inconsistent mouth directions")
    for t in wall:
        error = projected_cylinder(
            [vertices[v] for v in triangles[t]], bore_axis, bore_center, bore_radius, -1
        )
        require(error is not None, "residual region is not a concave cylindrical wall")
        residuals.append(error)
    require(max(residuals) <= FIT, "measured supports exceed vertex tolerance")
    return {
        "shaft_origin": center,
        "shaft_axis": axis,
        "height_mm": height,
        "shaft_radius_mm": radius,
        "bore_origin": bore_center,
        "bore_axis": bore_axis,
        "bore_radius_mm": bore_radius,
        "max_vertex_residual_mm": max(residuals),
        "source_regions": owners,
    }
