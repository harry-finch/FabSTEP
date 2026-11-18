"""Generate and check independent STL references for orthogonal cross drilling."""

import argparse
from collections import Counter, defaultdict
import hashlib
import json
import math
from pathlib import Path
import struct


ROOT = Path(__file__).resolve().parents[1] / "fixtures" / "cross_drill"
HALF_EXTENTS = (12., 10., 9.)
SMALL_RADIUS = 3.
LARGE_RADIUS = 6.5


def sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0])


def plane_label(axis, sign):
    return f"plane_{'xyz'[axis]}_{'positive' if sign > 0 else 'negative'}"


def pairs(loop):
    return zip(loop, loop[1:] + loop[:1])


class Mesh:
    def __init__(self):
        self.faces = []
        self.labels = []

    def triangle(self, a, b, c, normal, label):
        orientation = dot(cross(sub(b, a), sub(c, a)), normal)
        assert orientation != 0., (label, a, b, c)
        self.faces.append((a, b, c) if orientation > 0. else (a, c, b))
        self.labels.append(label)

    def quad(self, a, b, c, d, normal, label, alternate=False):
        if alternate:
            a, b, c, d = b, c, d, a
        self.triangle(a, b, c, normal, label)
        self.triangle(a, c, d, normal, label)


def right_semicircle(radius, intervals):
    """Samples from bottom to top, with the three cardinal points exact."""
    assert intervals % 2 == 0
    middle = intervals // 2
    points = [(0., -radius)]
    for i in range(1, intervals):
        angle = (i - middle) * math.pi / intervals
        points.append((radius * math.cos(angle), radius * math.sin(angle)))
    return points + [(0., radius)]


def complete_circle(right):
    return right + [(-x, y) for x, y in reversed(right[1:-1])]


def planar_face(mesh, half, axis, sign, hole=None, center=(0., 0.)):
    """Fan each convex inner quadrant to its corresponding outer corner."""
    j, k = (axis + 1) % 3, (axis + 2) % 3
    normal = tuple(float(sign) if i == axis else 0. for i in range(3))
    label = plane_label(axis, sign)

    def point(u, v):
        p = [0., 0., 0.]
        p[axis], p[j], p[k] = sign * half[axis], u, v
        return tuple(p)

    quadrants = [(1, 1), (-1, 1), (-1, -1), (1, -1)]
    corners = [point(u * half[j], v * half[k]) for u, v in quadrants]
    if hole is None:
        mesh.quad(*corners, normal, label)
        return
    cx, cy = center
    for a, b in pairs(hole):
        quadrant = (1 if a[j] + b[j] > 2. * cx else -1,
                    1 if a[k] + b[k] > 2. * cy else -1)
        mesh.triangle(a, b, corners[quadrants.index(quadrant)], normal, label)
    # The exact cardinal hole vertices connect adjacent corner fans.
    for index, (a, b) in enumerate(pairs(corners)):
        if index % 2 == 0:
            side = [p for p in hole if p[j] == cx and (p[k] > cy) == (a[k] > cy)]
        else:
            side = [p for p in hole if p[k] == cy and (p[j] > cx) == (a[j] > cx)]
        assert len(side) == 1, (label, index, side)
        mesh.triangle(a, b, side[0], normal, label)


def cross_drill(half, r, big_r, circumferential, steps, defect, small_steps):
    hx, _, hz = half
    cx, cy = (1.125, .625) if defect == "off_center" else (0., 0.)
    right = right_semicircle(r, circumferential // 2)
    if cy != 0.:
        # Include world y=0 exactly on both cylinders to keep the large
        # mouth's cardinal vertices in its ring even in this negative case.
        right.append((math.sqrt(r*r - cy*cy), -cy))
        right.sort(key=lambda p: p[1])
    small_ring = complete_circle(right)

    def seam_yz(y, sign):
        world_y = cy + y
        return world_y, sign * math.sqrt(big_r*big_r - world_y*world_y)

    mesh = Mesh()
    small_mouths = {}
    folded = None
    for sign in (-1, 1):
        label = f"small_bore_{'positive' if sign > 0 else 'negative'}"
        seam = [(cx + x, *seam_yz(y, sign)) for x, y in small_ring]
        mouth = [(x, y, sign * hz) for x, y, _ in seam]
        small_mouths[sign] = mouth
        levels = [seam]
        for i in range(1, small_steps):
            t = i / small_steps
            levels.append([(x, y, (1. - t) * z + t * sign * hz) for x, y, z in seam])
        levels.append(mouth)
        for i, (a, b) in enumerate(zip(levels, levels[1:])):
            for j in range(len(seam)):
                k = (j + 1) % len(seam)
                normal = (2. * cx - a[j][0] - a[k][0], 2. * cy - a[j][1] - a[k][1], 0.)
                mesh.quad(a[j], a[k], b[k], b[j], normal, label, alternate=(i + j) % 2 == 1)
        if defect == "fold" and sign > 0:
            row = levels[small_steps // 2]
            j = circumferential // 3
            p = row[j]
            # Keep the point on the analytic cylinder while moving it past the
            # next angular column. Connectivity and shared boundary stay put.
            moved = (*row[j + 2][:2], p[2] + .017)
            folded = (p, moved)

    # On each angular row of the large cylinder, x in ]cx-w,cx+w[ is
    # removed. The window ends collapse to one vertex; outside the windows
    # the two strips meet at x=cx. Both strips use exactly the seam vertices.
    rows = [(seam_yz(y, sign), x) for sign in (-1, 1) for x, y in right]
    extra = complete_circle(right_semicircle(big_r, circumferential // 2))
    rows.extend(((y, z), 0.) for y, z in extra if abs(y - cy) > r + 1e-12)
    rows.sort(key=lambda row: math.atan2(row[0][1], row[0][0]))
    large_mouths = {sign: [(sign * hx, *yz) for yz, _ in rows] for sign in (-1, 1)}

    def large_levels(row, sign):
        (y, z), width = row
        a, b = cx + sign * width, sign * hx
        return [(a, y, z)] + [(a + (b - a) * i / steps, y, z) for i in range(1, steps)] + [(b, y, z)]

    for i, (lower, upper) in enumerate(pairs(rows)):
        (ly, lz), _ = lower
        (uy, uz), _ = upper
        normal = (0., -ly - uy, -lz - uz)
        for sign in (-1, 1):
            a, b = large_levels(lower, sign), large_levels(upper, sign)
            for j in range(steps):
                mesh.quad(a[j], b[j], b[j + 1], a[j + 1], normal, "large_bore",
                          alternate=(i + j) % 2 == 1)

    for sign in (-1, 1):
        planar_face(mesh, half, 0, sign, large_mouths[sign])
        planar_face(mesh, half, 1, sign)
        if defect != "missing_mouth" or sign < 0:
            planar_face(mesh, half, 2, sign, small_mouths[sign], (cx, cy))
    if folded is not None:
        old, new = folded
        mesh.faces = [tuple(new if p == old else p for p in face) for face in mesh.faces]
        inverted = 0
        for (a, b, c), label in zip(mesh.faces, mesh.labels):
            if new in (a, b, c):
                assert label == "small_bore_positive"
                inward = (3. * cx - a[0] - b[0] - c[0], 3. * cy - a[1] - b[1] - c[1], 0.)
                inverted += dot(cross(sub(b, a), sub(c, a)), inward) < 0.
        assert inverted > 0
    return mesh


def placement(kind):
    if kind == "rotated":
        # Proper oblique rotation of the unit quaternion (1,2,2,0)/3.
        axes = ((1./9., 8./9., -4./9.), (8./9., 1./9., 4./9.), (4./9., -4./9., -7./9.))
        return axes, (9., -5., 11.), False
    if kind == "mirror":
        return ((-1., 0., 0.), (0., 1., 0.), (0., 0., 1.)), (4., -3., 2.), True
    return ((1., 0., 0.), (0., 1., 0.), (0., 0., 1.)), (0., 0., 0.), False


def place(point, axes, translation):
    return tuple(translation[i] + math.fsum(v * axis[i] for v, axis in zip(point, axes))
                 for i in range(3))


def write_stl(path, faces, ascii_stl):
    if ascii_stl:
        with path.open("w", encoding="ascii", newline="\n") as out:
            out.write(f"solid {path.stem}\n")
            for triangle in faces:
                out.write("facet normal 0 0 0\nouter loop\n")
                for p in triangle:
                    out.write("vertex " + " ".join(f"{x:.17g}" for x in p) + "\n")
                out.write("endloop\nendfacet\n")
            out.write(f"endsolid {path.stem}\n")
        return
    with path.open("wb") as out:
        out.write(bytes(80) + struct.pack("<I", len(faces)))
        for triangle in faces:
            out.write(struct.pack("<12fH", 0., 0., 0., *(v for p in triangle for v in p), 0))


def read_stl(path):
    data = path.read_bytes()
    if data.startswith(b"solid "):
        vertices = [tuple(map(float, line.split()[1:])) for line in data.decode("ascii").splitlines()
                    if line.startswith("vertex ")]
    else:
        count = struct.unpack_from("<I", data, 80)[0]
        assert len(data) == 84 + 50 * count
        vertices = []
        for offset in range(84, len(data), 50):
            row = struct.unpack_from("<12fH", data, offset)
            vertices.extend((row[3:6], row[6:9], row[9:12]))
    return list(zip(vertices[::3], vertices[1::3], vertices[2::3]))


def reachable(graph):
    pending = [next(iter(graph))]
    visited = set()
    while pending:
        point = pending.pop()
        if point not in visited:
            visited.add(point)
            pending.extend(set(graph[point]) - visited)
    return visited


def cycles(edges):
    graph = defaultdict(set)
    for a, b in edges:
        graph[a].add(b)
        graph[b].add(a)
    assert all(len(neighbors) == 2 for neighbors in graph.values())
    result = []
    while graph:
        cycle = reachable(graph)
        result.append(len(cycle))
        for vertex in cycle:
            del graph[vertex]
    return sorted(result)


def verify(faces, labels, open_case):
    """Check byte-reloaded topology without using the mesh generator's indices."""
    assert len(faces) == len(labels)
    edges = defaultdict(list)
    links = defaultdict(lambda: defaultdict(set))
    vertices = set()
    patch_edges = defaultdict(Counter)
    patch_vertices = defaultdict(set)
    neighbors = {i: set() for i in range(len(faces))}
    for i, (triangle, label) in enumerate(zip(faces, labels)):
        a, b, c = triangle
        normal = cross(sub(b, a), sub(c, a))
        assert dot(normal, normal) > 0.
        vertices.update(triangle)
        patch_vertices[label].update(triangle)
        for a, b, c in (triangle, (b, c, a), (c, a, b)):
            key = tuple(sorted((a, b)))
            edges[key].append((a, b, label, i))
            patch_edges[label][key] += 1
            links[a][b].add(c)
            links[a][c].add(b)
    joins = defaultdict(list)
    boundary = []
    for edge, uses in edges.items():
        if len(uses) == 1:
            boundary.append(edge)
            continue
        assert len(uses) == 2, edge
        (a, b, f, i), (rev_b, rev_a, g, j) = uses
        assert (a, b) == (rev_a, rev_b), (edge, f, g)
        neighbors[i].add(j)
        neighbors[j].add(i)
        if f != g:
            joins[tuple(sorted((f, g)))].append(edge)
    boundary_cycles = cycles(boundary)
    assert len(boundary_cycles) == (2 if open_case else 0)
    assert len(reachable(neighbors)) == len(faces)
    chi = len(vertices) - len(edges) + len(faces)
    assert chi == -4, (chi, boundary_cycles)
    boundary_vertices = {p for edge in boundary for p in edge}
    for vertex, link in links.items():
        assert len(reachable(link)) == len(link)
        degrees = sorted(map(len, link.values()))
        expected_ends = 2 if vertex in boundary_vertices else 0
        assert degrees == [1] * expected_ends + [2] * (len(link) - expected_ends)

    expected_curves = set()
    for side in ("negative", "positive"):
        small_wall = f"small_bore_{side}"
        expected_curves.add(("large_bore", small_wall))
        expected_curves.add(("large_bore", f"plane_x_{side}"))
        if not (open_case and side == "positive"):
            expected_curves.add((f"plane_z_{side}", small_wall))
    for pair, joined_edges in joins.items():
        if pair in expected_curves:
            assert len(cycles(joined_edges)) == 1
        else:
            assert all(label.startswith("plane_") for label in pair) and len(joined_edges) == 1
    assert expected_curves <= joins.keys()
    assert len(joins) == (13 if open_case else 18)

    counts = Counter(labels)
    patches = []
    first = 0
    for label, count in counts.items():
        assert labels[first:first + count] == [label] * count
        local_edges = patch_edges[label]
        boundaries = cycles([edge for edge, uses in local_edges.items() if uses == 1])
        patch_chi = len(patch_vertices[label]) - len(local_edges) + count
        expected_boundaries = 4 if label == "large_bore" else 1 if label.startswith("plane_y_") else 2
        assert len(boundaries) == expected_boundaries, (label, boundaries)
        assert patch_chi == 2 - expected_boundaries, (label, patch_chi)
        patches.append(dict(owner=label, first_source_triangle=first, triangles=count,
                            boundary_cycle_lengths=boundaries, euler_characteristic=patch_chi))
        first += count

    sampled_volume = math.fsum(dot(a, cross(b, c)) / 6. for a, b, c in faces)
    assert sampled_volume > 0.
    return (dict(euler_characteristic=chi, boundary_edges=len(boundary),
                 boundary_cycle_lengths=boundary_cycles, vertices=len(vertices),
                 sampled_volume_mm3=None if open_case else sampled_volume,
                 joined_edges=[[*pair, len(edges)] for pair, edges in sorted(joins.items())]), patches)


def ideal_volume(half, r, big_r):
    def integrand(angle):
        s, c = math.sin(angle), math.cos(angle)
        return 8. * r*r * c*c * math.sqrt(big_r*big_r - r*r*s*s)

    def simpson(intervals):
        dx = math.pi / (2. * intervals)
        total = math.fsum((4 if i % 2 else 2) * integrand(i * dx)
                          for i in range(1, intervals))
        return dx * (total + integrand(0.) + integrand(math.pi / 2.)) / 3.

    a, b = simpson(1024), simpson(2048)
    assert abs(a - b) < 1e-9
    hx, hy, hz = half
    void_sum = 2. * math.pi * (r*r*hz + big_r*big_r*hx)
    return 8. * hx * hy * hz - void_sum + b, b, abs(a - b)


def winding(faces, point):
    def solid_angle(face):
        a, b, c = (sub(p, point) for p in face)
        la, lb, lc = (math.sqrt(dot(p, p)) for p in (a, b, c))
        return 2. * math.atan2(dot(a, cross(b, c)),
                               la*lb*lc + dot(a, b)*lc + dot(b, c)*la + dot(c, a)*lb)
    return math.fsum(solid_angle(face) for face in faces) / (4. * math.pi)


def probes(half, r, big_r):
    hx, hy, hz = half
    values = [
        ((0., 0., 0.), False, "common_void"),
        ((.85 * hx, 0., 0.), False, "large_bore_positive"),
        ((-.85 * hx, 0., 0.), False, "large_bore_negative"),
        ((0., 0., .9 * hz), False, "small_bore_positive"),
        ((0., 0., -.9 * hz), False, "small_bore_negative"),
        ((.8 * hx, .9 * hy, .8 * hz), True, "block_corner"),
    ]
    # Stay clear of chord sag while probing across both support surfaces next
    # to their two branches, with |x| both inside and outside the thin bore.
    y = .55 * r
    x = math.sqrt(r*r - y*y)
    z = math.sqrt(big_r*big_r - y*y)
    gap = .03 * r
    for sign in (-1, 1):
        for point, expected, purpose in [
            ((x - gap, y, sign * (z + gap)), False, "small_bore_at_join"),
            ((x + gap, y, sign * (z - gap)), False, "large_bore_at_join"),
            ((x + gap, y, sign * (z + gap)), True, "material_at_join"),
        ]:
            values.append((point, expected, purpose))
    return values


def write_case(destination, name, scale, pose, circumferential, steps, defect, large_radius=LARGE_RADIUS, small_steps=None):
    half = tuple(scale * x for x in HALF_EXTENTS)
    r = scale * SMALL_RADIUS
    big_r = scale * large_radius
    mesh = cross_drill(half, r, big_r, circumferential, steps, defect,
                       steps if small_steps is None else small_steps)
    axes, center, flipped = placement(pose)
    faces = [tuple(place(p, axes, center) for p in (tri[::-1] if flipped else tri))
             for tri in mesh.faces]
    path = destination / f"{name}.stl"
    write_stl(path, faces, pose == "rotated")
    faces = read_stl(path)
    topology, patches = verify(faces, mesh.labels, defect == "missing_mouth")
    references = []
    bounds = volume = overlap = convergence = None
    if defect is None:
        corners = [place((sx * half[0], sy * half[1], sz * half[2]), axes, center)
                   for sx in (-1, 1) for sy in (-1, 1) for sz in (-1, 1)]
        bounds = [[extreme(p[i] for p in corners) for i in range(3)] for extreme in (min, max)]
        actual = [[extreme(p[i] for face in faces for p in face) for i in range(3)]
                  for extreme in (min, max)]
        assert all(abs(x - y) < 2e-6 for a, b in zip(bounds, actual) for x, y in zip(a, b))
        volume, overlap, convergence = ideal_volume(half, r, big_r)
        assert abs(topology["sampled_volume_mm3"] - volume) < .003 * volume
        for point, material, purpose in probes(half, r, big_r):
            assert (point[0]**2 + point[1]**2 > r*r and point[1]**2 + point[2]**2 > big_r*big_r) == material
            world = place(point, axes, center)
            assert abs(winding(faces, world) - float(material)) < 1e-7, (name, world, purpose)
            references.append(dict(point_mm=world, material=material, purpose=purpose))
    parameters = dict(dimensions_mm=[2. * x for x in half], center_mm=center,
                      small_radius_mm=r, large_radius_mm=big_r,
                      small_axis=axes[2], large_axis=axes[0],
                      small_axis_offset_local_mm=(1.125, .625, 0.) if defect == "off_center" else (0., 0., 0.),
                      volume_mm3=volume, void_overlap_mm3=overlap, volume_quadrature_difference_mm3=convergence,
                      analytic_faces=dict(plane=6, cylinder=3) if defect is None else None)
    return dict(sha256=hashlib.sha256(path.read_bytes()).hexdigest(), placement=pose,
                triangles=len(faces), positive=defect is None, defect=defect, parameters=parameters,
                analytic_bounds_mm=bounds, topology=topology, patches=patches, material_probes=references)


CASES = (
    ("f_nominal", 1., "nominal", 64, 3, None),
    ("f_scaled", 1.5, "nominal", 96, 3, None),
    ("f_rotated", 1., "rotated", 64, 3, None),
    ("f_mirror", 1., "mirror", 64, 3, None),
    ("f_retessellated", 1., "nominal", 88, 5, None),
    ("f_ratio", 1., "nominal", 64, 3, None, 5.5),
    ("f_ratio_mid", 1., "rotated", 80, 3, None, 5.0),
    ("f_ratio_strip", 1., "nominal", 80, 3, None, 5.0, 1),
    ("f_ratio_near", 1., "mirror", 96, 3, None, 4.51),
    ("f_ratio_boundary", 1., "nominal", 64, 3, "ratio_at_limit", 4.5),
    ("f_ratio_above", 1., "nominal", 64, 3, "ratio_above_limit", 4.49),
    ("f_off_center", 1., "nominal", 64, 3, "off_center"),
    ("f_fold", 1., "nominal", 64, 3, "fold"),
    ("f_missing_mouth", 1., "nominal", 64, 3, "missing_mouth"),
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, default=ROOT)
    args = parser.parse_args()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    cases = {case[0]: write_case(args.output_dir, *case) for case in CASES}
    (args.output_dir / "manifest.json").write_text(json.dumps(dict(units="mm", cases=cases), indent=2) + "\n",
                                                   encoding="utf-8")


if __name__ == "__main__":
    main()
