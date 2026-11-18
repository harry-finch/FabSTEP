"""Measure one concave cylindrical wall in a small closed pocket STL.

Laboratory scope: eleven dominant planar patches, two tangent supports, one
perpendicular stop and one oblique stop. No source groups or CAD model needed.
"""

import argparse
from collections import defaultdict
import json
import math
from pathlib import Path
import struct

import numpy as np


TOL = 1e-5
MAX_TRIANGLES = 4096


def require(condition, message):
    if not condition:
        raise ValueError(message)


def unit(vector):
    length = np.linalg.norm(vector)
    require(length > 1e-12, "singular direction")
    return vector / length


def load(path):
    require(path.stat().st_size <= 2 * 1024**2, "file exceeds probe limit")
    data = path.read_bytes()
    count = struct.unpack_from("<I", data, 80)[0] if len(data) >= 84 else 0
    if count and len(data) == 84 + count * 50:
        require(count <= MAX_TRIANGLES, "too many triangles")
        raw = [row[3:12] for row in struct.iter_unpack("<12fH", data[84:])]
    else:
        raw = [
            tuple(map(float, row.split()[1:]))
            for row in data.decode("ascii").splitlines()
            if row.lstrip().startswith("vertex ")
        ]
        require(
            0 < len(raw) <= 3 * MAX_TRIANGLES and len(raw) % 3 == 0, "invalid ASCII STL"
        )
    coordinates = np.array(raw, dtype=float).reshape(-1, 3)
    require(np.isfinite(coordinates).all(), "non-finite vertex")
    points, indices = np.unique(coordinates, axis=0, return_inverse=True)
    return points, indices.reshape(-1, 3)


def graph(points, triangles):
    uses = defaultdict(list)
    adjacent = [set() for _ in triangles]
    for i, (a, b, c) in enumerate(triangles):
        for p, q in [(a, b), (b, c), (c, a)]:
            uses[tuple(sorted((p, q)))].append((i, p < q))
    for edge in uses.values():
        require(
            len(edge) == 2 and edge[0][1] != edge[1][1],
            "open or unoriented source edge",
        )
        a, b = (item[0] for item in edge)
        adjacent[a].add(b)
        adjacent[b].add(a)
    a, b, c = points[triangles].transpose(1, 0, 2)
    cross = np.cross(b - a, c - a)
    twice_area = np.linalg.norm(cross, axis=1)
    require(min(twice_area) > 1e-12, "degenerate triangle")
    return uses, adjacent, cross / twice_area[:, None], twice_area


def flood(seed, adjacent, allowed):
    visited, queue = {seed}, [seed]
    while queue:
        for j in adjacent[queue.pop()]:
            if j not in visited and allowed(j):
                visited.add(j)
                queue.append(j)
    return visited


def split_planes(points, triangles, adjacent, normals, twice_area):
    # Domain-specific selection by area, followed by proofs on the residue.
    assigned, groups = set(), []
    for i in np.argsort(-twice_area):
        if i in assigned:
            continue
        normal = normals[i]
        origin = points[triangles[i, 0]]
        group = flood(
            i,
            adjacent,
            lambda j: j not in assigned
            and normals[j] @ normal > 0
            and np.max(np.abs((points[triangles[j]] - origin) @ normal)) <= TOL,
        )
        groups.append((twice_area[list(group)].sum(), group, i))
        assigned.update(group)
    groups.sort(key=lambda item: -item[0])
    require(
        len(groups) > 11 and groups[10][0] > 2 * groups[11][0],
        "ambiguous eleven dominant planes",
    )
    owner = np.full(len(triangles), -1)
    plane = []
    for index, (_, group, seed) in enumerate(groups[:11]):
        owner[list(group)] = index
        plane.append((points[triangles[seed, 0]], normals[seed]))
    wall = set(np.flatnonzero(owner < 0))
    require(
        wall == flood(min(wall), adjacent, wall.__contains__),
        "disconnected residual wall",
    )
    return wall, plane, owner


def path(edges):
    neighbors = defaultdict(set)
    for a, b in edges:
        neighbors[a].add(b)
        neighbors[b].add(a)
    ends = sorted(v for v, ns in neighbors.items() if len(ns) == 1)
    require(
        len(ends) == 2 and max(map(len, neighbors.values())) <= 2,
        "branched or cyclic boundary",
    )
    chain = ends[:1]
    while chain[-1] != ends[1]:
        others = neighbors[chain[-1]] - set(chain[-2:])
        require(len(others) == 1 and len(chain) <= len(edges), "broken boundary")
        chain.append(others.pop())
    require(len(chain) == len(edges) + 1, "disconnected boundary")
    return chain


def check(pathname, max_deviation=0.05):
    require(
        math.isfinite(max_deviation) and max_deviation > 0, "invalid deviation budget"
    )
    points, triangles = load(pathname)
    edges, adjacent, normals, areas = graph(points, triangles)
    require(
        len(flood(0, adjacent, lambda _: True)) == len(triangles), "disconnected source"
    )
    wall, plane, owner = split_planes(points, triangles, adjacent, normals, areas)
    boundary = defaultdict(list)
    wall_edges = 0
    for edge, uses in edges.items():
        a, b = (owner[i] for i, _ in uses)
        if min(a, b) == -1:
            wall_edges += 1
            if max(a, b) >= 0:
                boundary[max(a, b)].append(edge)
    chains = {key: path(edges) for key, edges in boundary.items()}
    require(len(chains) == 4, "wall needs four connected boundaries")
    vertices = np.unique(triangles[sorted(wall)])
    require(len(vertices) - wall_edges + len(wall) == 1, "residual is not a disk")
    rails, caps = [], []
    for index, chain in chains.items():
        p = points[chain]
        direction = unit(p[-1] - p[0])
        distance = np.linalg.norm(np.cross(p - p[0], direction), axis=1)
        (rails if max(distance) <= TOL else caps).append(index)
    require(len(rails) == len(caps) == 2, "expected two straight rails and two curves")
    n0, n1 = (plane[i][1] for i in rails)
    require(abs(n0 @ n1) < 1e-5, "expected perpendicular tangent supports")
    axis = unit(np.cross(n0, n1))
    circle, ellipse = sorted(caps, key=lambda i: -abs(plane[i][1] @ axis))
    require(
        abs(plane[circle][1] @ axis) > 1 - 1e-10
        and 0.1 < abs(plane[ellipse][1] @ axis) < 0.99999,
        "unqualified stop planes",
    )
    for rail in rails:
        ends = {chains[rail][0], chains[rail][-1]}
        for cap in caps:
            require(
                len(ends & {chains[cap][0], chains[cap][-1]}) == 1, "unshared corner"
            )
    near = points[chains[circle]]
    far = points[chains[ellipse]]
    if axis @ (far.mean(axis=0) - near.mean(axis=0)) < 0:
        axis = -axis
    mean = near.mean(axis=0)
    x = unit(near[0] - near[-1])
    y = unit(np.cross(axis, x))
    x = np.cross(y, axis)
    projected = np.column_stack(((near - mean) @ x, (near - mean) @ y))
    matrix = np.column_stack((2 * projected, np.ones(len(near))))
    solution, _, rank, _ = np.linalg.lstsq(
        matrix, np.sum(projected**2, axis=1), rcond=None
    )
    require(
        len(near) >= 7 and len(far) >= 7 and rank == 3,
        "under-resolved circular/elliptic stops",
    )
    center = mean + solution[0] * x + solution[1] * y
    radius = math.sqrt(solution[2] + solution[:2] @ solution[:2])
    x = unit(near[0] - center - axis * ((near[0] - center) @ axis))
    y = np.cross(axis, x)
    if y @ (near[-1] - center) < 0:
        y = -y
    theta = np.arctan2((points - center) @ y, (points - center) @ x)
    sweep = theta[chains[circle][-1]]
    require(abs(sweep - math.pi / 2) < 1e-5, "expected quarter cylinder")
    for rail in rails:
        angle = theta[chains[rail][0]]
        outward = x * np.cos(angle) + y * np.sin(angle)
        require(np.linalg.norm(outward + plane[rail][1]) < 2e-5, "missing tangency")
    plane_point, n = plane[ellipse]
    cosine = n @ axis
    h = ((plane_point - center) @ n) / cosine
    a, b = (-radius * (n @ tangent) / cosine for tangent in (x, y))
    amplitude = math.hypot(a, b)
    require(h > amplitude + TOL, "stop may intersect near end")
    height = h + a * np.cos(theta) + b * np.sin(theta)
    fraction = ((points - center) @ axis) / height
    for rail in rails:
        ids = chains[rail]
        side = round(np.mean(theta[ids]) / sweep)
        require(side in (0, 1), "wrong rail")
        theta[ids] = side * sweep
    for cap, station in ((circle, 0.0), (ellipse, 1.0)):
        ids = chains[cap]
        fraction[ids] = station
        differences = np.diff(theta[ids])
        require(np.all(differences > 0) or np.all(differences < 0), "backtracking cap")
    for rail in rails:
        differences = np.diff(fraction[chains[rail]])
        require(np.all(differences > 0) or np.all(differences < 0), "backtracking rail")
    height = h + a * np.cos(theta) + b * np.sin(theta)
    radii = np.outer(np.cos(theta), x) + np.outer(np.sin(theta), y)
    exact = center + radius * radii + np.outer(fraction * height, axis)
    vertex_error = np.linalg.norm(points - exact, axis=1)
    worst = max(vertex_error[vertices])
    require(worst <= TOL, f"wall/trim residual {worst:.7g} mm exceeds {TOL}")
    ts = triangles[sorted(wall)]
    normals_radial = np.einsum(
        "ij,ij->i", normals[sorted(wall)], radii[ts].mean(axis=1)
    )
    require(max(normals_radial) < 0, "inconsistent or convex wall")
    # Unroll the actual triangles in angle and axial distance. Their far edge
    # is the polygonal interpolant P of the exact stop height H. The four
    # simple monotone boundary chains and a consistent Jacobian give degree 1
    # inside this polygon and 0 outside. Compare its area independently.
    stops = sorted(chains[ellipse], key=theta.__getitem__)
    stations = theta[stops]
    poly_height = np.interp(theta[vertices], stations, height[stops])
    z = fraction * height
    uv = np.column_stack((theta, z))
    require(
        np.min(uv[vertices]) >= -1e-8
        and max(theta[vertices]) <= sweep + 1e-8
        and max(z[vertices] - poly_height) <= 1e-8,
        "vertex outside polygonal trim",
    )
    q, r, s = uv[ts].transpose(1, 0, 2)
    signed = (r[:, 0] - q[:, 0]) * (s[:, 1] - q[:, 1]) - (r[:, 1] - q[:, 1]) * (
        s[:, 0] - q[:, 0]
    )
    interval = np.diff(stations)
    polygon_area = np.sum(interval * (height[stops][1:] + height[stops][:-1])) / 2
    require(
        (min(signed) > 0 or max(signed) < 0)
        and abs(abs(sum(signed)) / 2 - polygon_area) < 1e-7,
        "folded or incomplete UV cover",
    )
    # Send (theta,z) to the exact strip at (theta,z*H/P). Since 0<=z<=P
    # and both heights are positive, this map is onto and displaces z by at
    # most ||H-P||. The second derivative of H is bounded by amplitude.
    # Angular barycentric interpolation of a circle differs by <= R*span²/8
    # (Taylor remainder and variance <= span²/4). These errors are orthogonal.
    stop_span = max(interval)
    axial = amplitude * stop_span**2 / 8
    span = np.ptp(theta[ts], axis=1)
    bound = max(
        np.hypot(radius * span**2 / 8, axial) + np.max(vertex_error[ts], axis=1)
    )
    major = radius / abs(cosine)
    stop_bound = major * stop_span**2 / 8 + max(vertex_error[chains[ellipse]])
    require(
        max(bound, stop_bound) <= max_deviation,
        f"wall or stop exceeds {max_deviation:.7g} mm; upper bound {max(bound, stop_bound):.7g} mm",
    )
    return dict(
        triangles=len(triangles),
        wall_triangles=len(wall),
        wall_source_triangles=sorted(map(int, wall)),
        cylinder_axis=axis.tolist(),
        radius_mm=radius,
        oblique_stop_segments=len(chains[ellipse]) - 1,
        circular_stop_segments=len(chains[circle]) - 1,
        ellipse_major_mm=major,
        ellipse_minor_mm=radius,
        circular_center_mm=center.tolist(),
        ellipse_center_mm=(center + h * axis).tolist(),
        vertex_residual_mm=worst,
        wall_bidirectional_bound_mm=bound,
        oblique_stop_chord_bound_mm=stop_bound,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("stl", type=Path)
    parser.add_argument(
        "--max-deviation", type=float, default=0.05, help="wall and stop budget in mm"
    )
    args = parser.parse_args()
    try:
        report = check(args.stl, args.max_deviation)
    except (ValueError, OSError, np.linalg.LinAlgError) as error:
        parser.exit(1, f"{args.stl}: {error}\n")
    print(json.dumps(report, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
