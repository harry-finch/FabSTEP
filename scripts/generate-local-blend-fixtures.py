"""Independent D and G source meshes, using Python's standard library only."""

import argparse
from collections import Counter, defaultdict
import hashlib
import json
import math
from pathlib import Path
import struct


LENGTHS = (20., 16., 10.)
RADIUS = 2.
ROOT = Path(__file__).resolve().parents[1] / "fixtures" / "local_blends"


def sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0])


def coordinate(p, axis, value):
    return tuple(value if i == axis else x for i, x in enumerate(p))


def direction(axis, sign):
    return coordinate((0., 0., 0.), axis, sign)


class Mesh:
    def __init__(self):
        self.vertices = []
        self.index = {}
        self.triangles = []
        self.labels = []

    def vertex(self, p):
        if p not in self.index:
            self.index[p] = len(self.vertices)
            self.vertices.append(p)
        return self.index[p]

    def triangle(self, points, outward, label):
        a, b, c = points
        orientation = dot(cross(sub(b, a), sub(c, a)), outward)
        if not math.isfinite(orientation) or orientation == 0.:
            raise ValueError(f"Degenerate face {label}: {points}")
        if orientation < 0.:
            points = (a, c, b)
        self.triangles.append(tuple(self.vertex(p) for p in points))
        self.labels.append(label)

    def fan(self, boundary, outward, label):
        center = tuple(math.fsum(p[i] for p in boundary) / len(boundary) for i in range(3))
        for a, b in zip(boundary, boundary[1:] + boundary[:1]):
            self.triangle((center, a, b), outward, label)

    def strip(self, lower, upper, outward, label):
        assert len(lower) == len(upper)
        for i in range(len(lower) - 1):
            self.triangle((lower[i], lower[i + 1], upper[i + 1]), outward, label)
            self.triangle((lower[i], upper[i + 1], upper[i]), outward, label)


def corner(n, axial_steps=3, defect=None):
    mesh = Mesh()
    sphere = {}
    r = RADIUS
    for i in range(n + 1):
        for j in range(n - i + 1):
            k = n - i - j
            length = math.sqrt(i*i + j*j + k*k)
            radius = r
            if defect == "sphere_dent" and (i, j) == (n // 3, n // 3):
                radius -= .04
            sphere[i, j, k] = tuple(r - radius * h / length for h in (i, j, k))

    for i in range(n):
        for j in range(n - i):
            k = n - i - j
            a, b, c = sphere[i, j, k], sphere[i + 1, j, k - 1], sphere[i, j + 1, k - 1]
            mesh.triangle((a, b, c), (-1., -1., -1.), "sphere_octant")
            if k >= 2:
                d = sphere[i + 1, j + 1, k - 2]
                mesh.triangle((b, d, c), (-1., -1., -1.), "sphere_octant")

    profiles = []
    slices = []
    for axis, length in enumerate(LENGTHS):
        j, k = (axis + 1) % 3, (axis + 2) % 3
        edge = []
        for i in range(n + 1):
            index = [0, 0, 0]
            index[j], index[k] = i, n - i
            edge.append(sphere[tuple(index)])
        levels = [r + (length - r) * i / axial_steps for i in range(axial_steps + 1)]
        levels[-1] = length
        rings = [[coordinate(p, axis, z) for p in edge] for z in levels]
        slices.append(levels)
        profiles.append(rings)
        for lower, upper in zip(rings, rings[1:]):
            mesh.strip(lower, upper, coordinate((-1., -1., -1.), axis, 0.), f"cylinder_{axis}")

    for axis, length in enumerate(LENGTHS):
        j, k = (axis + 1) % 3, (axis + 2) % 3

        def p(fixed, y, z):
            out = [0., 0., 0.]
            out[axis], out[j], out[k] = fixed, y, z
            return tuple(out)

        far = [p(length, LENGTHS[j], 0.), p(length, LENGTHS[j], LENGTHS[k]),
               p(length, 0., LENGTHS[k]), *reversed(profiles[axis][-1])]
        mesh.fan(far, direction(axis, 1.), f"plane_{axis}_far")
        if defect != "missing_plane" or axis != 2:
            low = [*[p(0., y, r) for y in slices[j]], p(0., LENGTHS[j], LENGTHS[k]),
                   *[p(0., r, z) for z in reversed(slices[k][1:])]]
            mesh.fan(low, direction(axis, -1.), f"plane_{axis}_near")
    return mesh


def r_z(z, a, b, defect):
    if defect == "nonlinear":
        return a + b*z + .2 * math.sin(math.pi * z / LENGTHS[2])
    return a + b*z


def envelope_point(s, z, a, b, defect=None):
    r = r_z(z, a, b, defect)
    k = math.sqrt(2.) if defect == "circular_stack" else math.sqrt(2. * (1. + b*b))
    t = 1. - s
    d = t*t + s*s + k * s*t
    x, y = r * t*t / d, r * s*s / d
    assert abs(x + y + k * math.sqrt(x*y) - r) < 1e-12
    if defect != "circular_stack" and defect != "nonlinear":
        # Stationary sphere parameter from dF/dt=0. Check F=0 directly.
        shift = b * (x + y - r) / (1. + b*b)
        sphere_radius = a + b * (z + shift)
        residual = (x - sphere_radius)**2 + (y - sphere_radius)**2 + shift**2 - sphere_radius**2
        assert abs(residual) < 1e-12
        # Check the same point independently against the circular cone form.
        q = math.sqrt(1. + 2.*b*b)
        axis = tuple(math.copysign(1., b) * u / q for u in (b, b, 1.))
        vector = (x, y, z + a/b)
        axial = dot(vector, axis)
        perpendicular = cross(vector, axis)
        radial = math.sqrt(dot(perpendicular, perpendicular))
        assert axial > 0.
        assert abs(radial - axial * math.tan(math.asin(abs(b)/q))) < 1e-12
    return x, y, z


def envelope(a, b, n, axial_steps, defect=None, staggered=False):
    mesh = Mesh()
    length, width, height = LENGTHS
    rings = []
    for level in range(axial_steps + 1):
        z = height * level / axial_steps
        ring = []
        for i in range(n + 1):
            s = i / n
            if staggered and 0 < level < axial_steps and 0 < i < n:
                s += .28 / n * math.sin(math.pi * s) * (-1.)**level
            ring.append(envelope_point(s, z, a, b, defect))
        rings.append(ring)
    for lower, upper in zip(rings, rings[1:]):
        mesh.strip(lower, upper, (-1., -1., 0.), "inclined_cone")

    for index, outward in ((0, (0., 0., -1.)), (-1, (0., 0., 1.))):
        ring = rings[index]
        z = ring[0][2]
        cap = [*ring, (0., width, z), (length, width, z), (length, 0., z)]
        if defect != "missing_cap" or index != -1:
            mesh.fan(cap, outward, "cap_bottom" if index == 0 else "cap_top")
    for ring_index, axis, far, label in ((0, 1, length, "tangent_y0"),
                                         (-1, 0, width, "tangent_x0")):
        contact = [ring[ring_index] for ring in rings]
        edge = [coordinate(p, 1 - axis, far) for p in contact]
        mesh.fan([*contact, *reversed(edge)], direction(axis, -1.), label)
    # Keep the same source z subdivisions where the far planes meet the rails.
    far_x = [(length, 0., ring[0][2]) for ring in rings]
    far_y = [(0., width, ring[0][2]) for ring in rings]
    mesh.fan([*far_x, (length, width, height), (length, width, 0.)], (1., 0., 0.), "far_x")
    mesh.fan([*far_y, (length, width, height), (length, width, 0.)], (0., 1., 0.), "far_y")
    return mesh


def shape_bounds(placement, parameters):
    f, _ = transform(placement)
    x, y, z = LENGTHS
    points = [f((i, j, k)) for i in (0., x) for j in (0., y) for k in (0., z)]
    low, high = [list(extreme(p[i] for p in points) for i in range(3)) for extreme in (min, max)]
    if placement == "oblique":
        # Only the positive mixture .6*x+.8*y loses its bounding point.
        if parameters["family"] == "trihedral":
            low[1] += RADIUS * (.6 + .8 - math.hypot(.6, .8))
        else:
            a, b = parameters["intercept_mm"], parameters["slope"]
            radius = min(a, a + b*z)
            center = radius / (1. - b*b)
            major = radius * math.sqrt(1. + b*b) / (1. - b*b)
            minor = radius / math.sqrt(1. - b*b)
            support = math.hypot(major * 1.4, minor * .2) / math.sqrt(2.)
            low[1] += center * 1.4 - support
    return [low, high]


def transform(placement):
    if placement == "mirrored":
        return lambda p: (LENGTHS[0] - p[0], p[1], p[2]), True
    if placement == "permuted":
        return lambda p: (p[2] + 3., p[0] - 8., p[1] + 5.), False
    if placement == "oblique":
        return lambda p: ((4*p[0] - 3*p[1]) / 5. + 13.,
                          (3*p[0] + 4*p[1]) / 5. - 7., p[2] + 11.), False
    return lambda p: p, False


def volume_d():
    # Edge strips and corner cube have disjoint interiors before rounding.
    r = RADIUS
    edge_loss = r*r * (1. - math.pi/4.) * (sum(LENGTHS) - 3.*r)
    corner_loss = r**3 * (1. - math.pi/6.)
    return math.prod(LENGTHS) - edge_loss - corner_loss


def area_coefficient(b):
    k = math.sqrt(2. * (1. + b*b))
    h = math.sqrt(1. - k*k/4.)
    analytic = (2. - k * math.acos(k/2.) / h) / (4. - k*k)
    # Green's formula for the rational parameterization gives the independently
    # evaluated integrand s(1-s)/D^2. Simpson is only a numeric cross-check.
    n = 4096
    def f(s):
        t = 1. - s
        return s*t / (t*t + s*s + k*s*t)**2
    numerical = math.fsum((4. if i % 2 else 2.) * f(i / n) for i in range(1, n)) / (3*n)
    assert abs(analytic - numerical) < 1e-12
    return analytic


def volume_g(a, b):
    height = LENGTHS[2]
    end = a + b*height
    return math.prod(LENGTHS) - area_coefficient(b) * height * (a*a + a*end + end*end)/3.


def corner_reference(placement):
    position, _ = transform(placement)
    center = (RADIUS,) * 3
    return dict(sphere_center_mm=position(center),
                cylinder_center_segments_mm=[
                    [position(center), position(coordinate(center, axis, length))]
                    for axis, length in enumerate(LENGTHS)])


def envelope_reference(placement, a, b):
    position, _ = transform(placement)
    q = math.sqrt(1. + 2.*b*b)
    direction = tuple(math.copysign(1., b) * u / q for u in (b, b, 1.))
    axis = sub(position(direction), position((0., 0., 0.)))
    caps = []
    for z in (0., LENGTHS[2]):
        r = a + b*z
        c = r / (1. - b*b)
        caps.append(dict(center_mm=position((c, c, z)),
                         semimajor_mm=r * math.sqrt(1. + b*b) / (1. - b*b),
                         semiminor_mm=r / math.sqrt(1. - b*b),
                         sweep_rad=2. * math.asin(math.sqrt((1. - b*b)/2.)),
                         endpoints_mm=[position((r, 0., z)), position((0., r, z))]))
    return dict(cone_apex_mm=position((0., 0., -a/b)), cone_axis=axis,
                cone_semi_angle_rad=math.asin(abs(b)/q), ellipse_caps=caps)


def probes_d():
    for t in (.35, .85):
        v = RADIUS * (1. - t)
        yield (v, v, v), 3*t*t < 1., "octant"
        for axis, length in enumerate(LENGTHS):
            yield coordinate((v, v, v), axis, .6*length), 2*t*t < 1., f"cylinder_{axis}"


def probes_g(a, b):
    for z in (1., 5., 9.):
        r = a + b*z
        boundary = r / (2. + math.sqrt(2. * (1. + b*b)))
        for delta in (-.015, .015):
            p = boundary + delta
            yield (p, p, z), delta > 0., "rolling_envelope"
        between = (boundary + r / (2. + math.sqrt(2.))) / 2.
        yield (between, between, z), True, "discriminates_circular_stack"


def polyhedron_winding(vertices, triangles, point):
    def solid_angle(triangle):
        a, b, c = (sub(vertices[i], point) for i in triangle)
        la, lb, lc = (math.sqrt(dot(v, v)) for v in (a, b, c))
        return 2.*math.atan2(dot(a, cross(b, c)),
                             la*lb*lc + dot(a, b)*lc + dot(b, c)*la + dot(c, a)*lb)
    return math.fsum(solid_angle(t) for t in triangles) / (4.*math.pi)


def verify(path, labels, *, open_case):
    data = path.read_bytes()
    if data.startswith(b"solid "):
        points = [tuple(map(float, line.split()[1:])) for line in data.decode().splitlines()
                  if line.startswith("vertex ")]
    else:
        points = []
        count = struct.unpack_from("<I", data, 80)[0]
        assert len(data) == 84 + 50*count
        for start in range(84, len(data), 50):
            p = struct.unpack_from("<12fH", data, start)
            points.extend((p[3:6], p[6:9], p[9:12]))
    vertices = list(dict.fromkeys(points))
    index = {p: i for i, p in enumerate(vertices)}
    triangles = [tuple(index[p] for p in t) for t in zip(points[::3], points[1::3], points[2::3])]
    assert len(labels) == len(triangles)
    edges = defaultdict(list)
    links = defaultdict(lambda: defaultdict(set))
    for t, face in zip(triangles, labels):
        p, q, r = (vertices[i] for i in t)
        normal = cross(sub(q, p), sub(r, p))
        assert dot(normal, normal) > 0.
        for a, b, c in zip(t, t[1:] + t[:1], t[2:] + t[:2]):
            edges[min(a, b), max(a, b)].append((a, b, face))
            links[a][b].add(c)
            links[a][c].add(b)
    joins = Counter()
    boundary = 0
    for uses in edges.values():
        if len(uses) == 1:
            boundary += 1
        else:
            assert len(uses) == 2
            (a, b, f), (reverse_b, reverse_a, g) = uses
            assert (a, b) == (reverse_a, reverse_b)
            if f != g:
                joins[tuple(sorted((f, g)))] += 1
    assert (boundary > 0) == open_case
    chi = len(vertices) - len(edges) + len(triangles)
    if not open_case:
        assert chi == 2
        for link in links.values():
            start = next(iter(link))
            visited = set()
            pending = [start]
            while pending:
                point = pending.pop()
                if point not in visited:
                    visited.add(point)
                    assert len(link[point]) == 2
                    pending.extend(link[point] - visited)
            assert len(visited) == len(link)
    volume = math.fsum(dot(vertices[a], cross(vertices[b], vertices[c])) / 6.
                       for a, b, c in triangles)
    assert volume > 0.
    return vertices, triangles, dict(euler_characteristic=chi, boundary_edges=boundary,
                                     sampled_volume_mm3=volume,
                                     joined_edges=[[*pair, count] for pair, count in sorted(joins.items())])


def write(root, name, mesh, placement, positive, parameters, probes=(), defect=None):
    position, flip = transform(placement)
    vertices = [position(p) for p in mesh.vertices]
    triangles = [(c, b, a) for a, b, c in mesh.triangles] if flip else mesh.triangles
    path = root / f"{name}.stl"
    if placement == "oblique":
        with path.open("w") as out:
            out.write(f"solid {name}\n")
            for t in triangles:
                out.write("facet normal 0 0 0\nouter loop\n")
                for i in t:
                    out.write("vertex " + " ".join(f"{x:.17g}" for x in vertices[i]) + "\n")
                out.write("endloop\nendfacet\n")
            out.write(f"endsolid {name}\n")
    else:
        with path.open("wb") as out:
            out.write(bytes(80) + struct.pack("<I", len(triangles)))
            for t in triangles:
                coords = [x for i in t for x in vertices[i]]
                out.write(struct.pack("<12fH", 0., 0., 0., *coords, 0))
    points, faces, topology = verify(path, mesh.labels, open_case=defect in ("missing_plane", "missing_cap"))
    transformed_probes = []
    for p, material, purpose in probes:
        point = position(p)
        winding = polyhedron_winding(points, faces, point)
        assert abs(winding - float(material)) < 1e-7, (name, point, purpose, winding)
        transformed_probes.append(dict(point_mm=point, material=material, purpose=purpose))
    if positive:
        bounds = [tuple(fn(p[i] for p in points) for i in range(3)) for fn in (min, max)]
        expected = shape_bounds(placement, parameters)
        tolerance = .004 if placement == "oblique" else 2e-6
        assert all(abs(a - b) <= tolerance for end in zip(bounds, expected) for a, b in zip(*end))
        assert abs(topology["sampled_volume_mm3"] - parameters["volume_mm3"]) < 1.
    counts = Counter(mesh.labels)
    patches = []
    start = 0
    for label, count in counts.items():
        assert all(mesh.labels[i] == label for i in range(start, start + count))
        patches.append(dict(owner=label, first_source_triangle=start, triangles=count))
        start += count
    return dict(sha256=hashlib.sha256(path.read_bytes()).hexdigest(), placement=placement,
                triangles=len(faces), positive=positive, defect=defect, parameters=parameters,
                analytic_bounds_mm=shape_bounds(placement, parameters) if positive else None,
                topology=topology, patches=patches, material_probes=transformed_probes)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, default=ROOT)
    root = parser.parse_args().output_dir
    root.mkdir(parents=True, exist_ok=True)
    cases = {}
    for name, n, steps, placement, defect in (
        ("d_corner", 16, 3, "plain", None), ("d_corner_mirror", 16, 3, "mirrored", None),
        ("d_corner_permuted", 20, 4, "permuted", None),
        ("d_corner_oblique", 24, 5, "oblique", None),
        ("d_corner_dent", 16, 3, "plain", "sphere_dent"),
        ("d_corner_missing_plane", 16, 3, "plain", "missing_plane"),
    ):
        parameters = dict(family="trihedral", dimensions_mm=LENGTHS, radius_mm=RADIUS,
                          arc_intervals=n, axial_steps=steps)
        if defect is None:
            parameters.update(volume_mm3=volume_d(), analytic_faces=dict(plane=6, cylinder=3, sphere=1),
                              circle_joins=3, line_tangencies=6, circle_ends=3,
                              shape=corner_reference(placement))
        cases[name] = write(root, name, corner(n, steps, defect), placement, defect is None,
                            parameters, probes_d() if defect is None else (), defect)
    for name, a, b, n, steps, placement, stagger, defect in (
        ("g_increasing", 3., .2, 24, 5, "plain", False, None),
        ("g_decreasing", 5., -.2, 24, 5, "plain", False, None),
        ("g_mirror", 3., .2, 24, 5, "mirrored", False, None),
        ("g_permuted", 3., .2, 28, 7, "permuted", False, None),
        ("g_oblique_staggered", 3., .2, 32, 8, "oblique", True, None),
        ("g_circular_stack", 3., .2, 24, 5, "plain", False, "circular_stack"),
        ("g_nonlinear", 3., .2, 24, 5, "plain", False, "nonlinear"),
        ("g_parabolic_cap", 3., 1., 24, 5, "plain", False, "parabolic_cap"),
        ("g_missing_cap", 3., .2, 24, 5, "plain", False, "missing_cap"),
    ):
        parameters = dict(family="rolling_envelope", dimensions_mm=LENGTHS, intercept_mm=a,
                          slope=b, ellipse_intervals=n, axial_steps=steps, staggered=stagger)
        if defect is None:
            parameters.update(volume_mm3=volume_g(a, b), analytic_faces=dict(plane=6, cone=1),
                              ellipse_ends=2, line_tangencies=2,
                              shape=envelope_reference(placement, a, b))
        probes = ()
        if defect is None:
            probes = probes_g(a, b)
        elif defect == "circular_stack":
            probes = [(p, False, purpose) for p, _, purpose in probes_g(a, b)
                      if purpose == "discriminates_circular_stack"]
        cases[name] = write(root, name, envelope(a, b, n, steps, defect, stagger), placement,
                            defect is None, parameters, probes, defect)
    (root / "manifest.json").write_text(json.dumps(dict(units="mm", cases=cases), indent=2) + "\n")


if __name__ == "__main__":
    main()
