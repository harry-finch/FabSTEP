#!/usr/bin/env python3
"""Generate an independent corpus of capped cubic revolutions in millimetres."""

from collections import Counter, defaultdict
from itertools import pairwise
import json
import math
from pathlib import Path
import struct


ROOT = Path(__file__).resolve().parents[1] / "fixtures" / "smooth_revolution"
IDENTITY = ((1., 0., 0.), (0., 1., 0.), (0., 0., 1.))
# Columns of the proper rotation from the unit quaternion (1, 2, 2, 0)/3.
OBLIQUE = ((1./9., 8./9., -4./9.), (8./9., 1./9., 4./9.), (4./9., -4./9., -7./9.))
MIRROR = ((1., 0., 0.), (0., 1., 0.), (0., 0., -1.))
BULGE = (5., 9., 9., 5.)
NECK = (8., 3., 3., 8.)
INFLECTION = (4., 9., 3., 8.)


def sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


def dot(a, b):
    return math.fsum(x * y for x, y in zip(a, b))


def cross(a, b):
    i, j, k = a
    x, y, z = b
    return (j*z - k*y, k*x - i*z, i*y - j*x)


def reachable(graph):
    pending = [next(iter(graph))]
    visited = set()
    while pending:
        node = pending.pop()
        if node not in visited:
            visited.add(node)
            pending.extend(graph[node] - visited)
    return visited


def verify(faces, missing_cap):
    """Use decoded coordinates to verify the topology represented by the STL bytes."""
    edges = defaultdict(list)
    links = defaultdict(lambda: defaultdict(set))
    graph = defaultdict(set)
    volumes = []
    for i, face in enumerate(faces):
        a, b, c = face
        assert any(cross(sub(b, a), sub(c, a)))
        volumes.append(dot(a, cross(b, c)) / 6.)
        graph[i]
        for p, q, r in [face, (b, c, a), (c, a, b)]:
            edges[tuple(sorted((p, q)))].append((p < q, i))
            links[p][q].add(r)
            links[p][r].add(q)
    boundary = defaultdict(set)
    for edge, incidences in edges.items():
        if len(incidences) == 1 and missing_cap:
            a, b = edge
            boundary[a].add(b)
            boundary[b].add(a)
        else:
            assert len(incidences) == 2
            (forward, first), (reverse, second) = incidences
            assert forward != reverse
            graph[first].add(second)
            graph[second].add(first)
    assert len(reachable(graph)) == len(faces)
    if missing_cap:
        assert len(reachable(boundary)) == len(boundary)
        assert all(len(neighbors) == 2 for neighbors in boundary.values())
    assert len(links) - len(edges) + len(faces) == (1 if missing_cap else 2)
    for vertex, link in links.items():
        assert len(reachable(link)) == len(link)
        assert sorted(map(len, link.values())) == (
            [1, 1] + [2] * (len(link) - 2) if vertex in boundary else [2] * len(link)
        )
    volume = math.fsum(volumes)
    assert volume > 0.
    return {"vertices": len(links), "boundary_edges": len(boundary), "signed_volume_mm3": volume}


def write_stl(path, faces, ascii_stl):
    if ascii_stl:
        with path.open("w", encoding="ascii", newline="\n") as out:
            out.write(f"solid {path.stem}\n")
            for triangle in faces:
                out.write("facet normal 0 0 0\nouter loop\n")
                for point in triangle:
                    out.write("vertex " + " ".join(f"{x:.17g}" for x in point) + "\n")
                out.write("endloop\nendfacet\n")
            out.write(f"endsolid {path.stem}\n")
        points = [tuple(map(float, line.split()[1:]))
                  for line in path.read_text(encoding="ascii").splitlines() if line.startswith("vertex ")]
    else:
        with path.open("wb") as out:
            out.write(bytes(80) + struct.pack("<I", len(faces)))
            for triangle in faces:
                out.write(struct.pack("<12fH", 0., 0., 0., *(x for p in triangle for x in p), 0))
        data = path.read_bytes()
        assert len(data) == 84 + 50 * len(faces)
        assert struct.unpack_from("<I", data, 80)[0] == len(faces)
        points = []
        for offset in range(84, len(data), 50):
            row = struct.unpack_from("<12fH", data, offset)
            points.extend((row[3:6], row[6:9], row[9:12]))
    return list(zip(points[::3], points[1::3], points[2::3]))


def cubic_radius(radii, t):
    # De Casteljau evaluation, independent of the fitter and of bounds below.
    while len(radii) > 1:
        radii = tuple((1. - t) * a + t * b for a, b in pairwise(radii))
    return radii[0]


def real_roots(a, b, c):
    if a == 0.:
        return [] if b == 0. else [-c / b]
    d = b*b - 4.*a*c
    if d < 0.:
        return []
    return [(-b - math.sqrt(d)) / (2.*a), (-b + math.sqrt(d)) / (2.*a)]


def analytic_bounds(radii, height, axis, translation):
    a, b, c, d = radii
    cubic = d - 3.*c + 3.*b - a
    quadratic = 3.*(a - 2.*b + c)
    linear = 3.*(b - a)
    corners = [[], []]
    for direction, origin in zip(axis, translation):
        for sign, corner, extreme in [(-1., corners[0], min), (1., corners[1], max)]:
            radial_extent = sign * math.sqrt(max(0., 1. - direction**2))
            slope = direction * height
            stations = real_roots(3.*cubic * radial_extent, 2.*quadratic * radial_extent,
                                  slope + linear * radial_extent)
            candidates = [0., 1.] + [t for t in stations if 0. < t < 1.]
            corner.append(extreme(origin + slope * t + radial_extent * cubic_radius(radii, t)
                                  for t in candidates))
    return corners


def generate(name, radii, height, *, divisions=32, angles=64, axes=IDENTITY,
             translation=(0., 0., 0.), scale=1., phase=0., defect=None):
    radii = tuple(r * scale for r in radii)
    height *= scale
    mirrored = dot(cross(axes[0], axes[1]), axes[2]) < 0.

    def placed(point):
        return tuple(origin + dot(point, components) for origin, components in zip(translation, zip(*axes)))

    def radius(t):
        r = cubic_radius(radii, t)
        return r + 64. * t**2 * (1. - t)**2 if defect == "quartic" else r

    def vertex(t, column):
        angle = math.tau * column / angles + phase
        r = radius(t)
        return (r * math.cos(angle), r * math.sin(angle), height * t)

    t_stations = [i / divisions for i in range(divisions + 1)]
    rows = [[vertex(t, j) for j in range(angles)] for t in t_stations]
    if defect == "incomplete_level":
        i, j = divisions // 2, angles // 3
        rows[i][j] = vertex((i + 0.375) / divisions, j)
    if defect == "folded":
        i, j = divisions // 2, angles // 3
        row = rows[i]
        row[j] = vertex(t_stations[i], j + 1.6)
        # A local triangle turns inward while the meridian residual stays zero.
        p, q, r = row[j], row[j + 1], rows[i + 1][j + 1]
        outward = (q[0] + p[0], q[1] + p[1], 0.)
        assert dot(cross(sub(q, p), sub(r, p)), outward) < 0.

    faces = []
    patches = []

    def patch(owner, triangles):
        first = len(faces)
        for triangle in triangles:
            a, b, c = map(placed, triangle)
            faces.append((a, c, b) if mirrored else (a, b, c))
        patches.append({"owner": owner, "first_source_triangle": first, "triangles": len(faces) - first})

    def wall():
        for i, (lower, upper) in enumerate(pairwise(rows)):
            for j in range(angles):
                k = (j + 1) % angles
                a, b, c, d = lower[j], lower[k], upper[k], upper[j]
                if defect == "axial_tip" and i == 0:
                    yield a, c, d
                elif (i + j) % 2:
                    yield a, b, d
                    yield b, c, d
                else:
                    yield a, b, c
                    yield a, c, d

    if defect != "axial_tip":
        patch("lower_cap", (((0., 0., 0.), rows[0][(j+1) % angles], rows[0][j]) for j in range(angles)))
    patch("wall", wall())
    missing_cap = defect == "missing_cap"
    if not missing_cap:
        patch("upper_cap", (((0., 0., height), rows[-1][j], rows[-1][(j+1) % angles]) for j in range(angles)))
    path = ROOT / f"{name}.stl"
    encoded_faces = write_stl(path, faces, ascii_stl=axes == OBLIQUE)
    topology = verify(encoded_faces, missing_cap)
    assert topology["boundary_edges"] == (angles if missing_cap else 0)
    case = {"triangles": len(faces), "axial_divisions": divisions, "azimuths": angles,
            "patches": patches, "topology": topology}
    if defect is not None:
        case["defect"] = defect
        return case

    # Composite Simpson integration of the cross-sectional areas.
    intervals = 4096
    weighted = (weight * radius(i / intervals)**2 for i in range(intervals + 1)
                for weight in [1 if i in (0, intervals) else 4 if i % 2 else 2])
    volume = math.pi * height * math.fsum(weighted) / (3. * intervals)
    case.update({"parameters": {"radii_mm": radii, "height_mm": height, "origin_mm": translation,
                                "axis": axes[2], "volume_mm3": volume},
                 "analytic_bounds_mm": analytic_bounds(radii, height, axes[2], translation),
                 "material_probes": [
                     {"point_mm": placed((radius(t) + gap * scale, 0., height * t)), "material": gap < 0.}
                     for t in [0.17, 0.39, 0.65, 0.89] for gap in [-0.25, 0.25]
                 ] + [{"point_mm": placed((0., 0., t * height)), "material": False} for t in [-0.05, 1.05]]})
    return case


def main():
    ROOT.mkdir(parents=True, exist_ok=True)
    cases = {
        "bulge": generate("bulge", BULGE, 20.),
        "neck": generate("neck", NECK, 20.),
        "inflection": generate("inflection", INFLECTION, 24.),
        "rotated": generate("rotated", INFLECTION, 24., axes=OBLIQUE, translation=(9., -5., 11.)),
        "scaled": generate("scaled", BULGE, 20., scale=1.5),
        "mirrored": generate("mirrored", INFLECTION, 24., axes=MIRROR, translation=(4., -3., 2.)),
        "retessellated": generate("retessellated", NECK, 20., divisions=40, angles=80, phase=0.013),
    }
    negative_cases = {
        "missing_cap": generate("missing_cap", BULGE, 20., defect="missing_cap"),
        "folded": generate("folded", BULGE, 20., defect="folded"),
        "incomplete_level": generate("incomplete_level", BULGE, 20., defect="incomplete_level"),
        "sparse": generate("sparse", BULGE, 20., divisions=3, defect="sparse"),
        "quartic": generate("quartic", BULGE, 20., defect="quartic"),
        "axial_tip": generate("axial_tip", (0., 4., 2., 8.), 20., defect="axial_tip"),
    }
    (ROOT / "manifest.json").write_text(json.dumps({"cases": cases, "negative_cases": negative_cases},
                                                 indent=2, allow_nan=False) + "\n", encoding="ascii")


if __name__ == "__main__":
    main()
