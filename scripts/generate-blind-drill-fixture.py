"""Regression fixture: four separated conical-tip blind bores on two axes."""

from collections import defaultdict
from pathlib import Path
import runpy


HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
G = runpy.run_path(str(ROOT / "scripts/generate-cross-drill-fixtures.py"))
mesh = G["Mesh"]()
half = G["HALF_EXTENTS"]

for axis in (0, 2):
    j, k = (axis + 1) % 3, (axis + 2) % 3
    radius = G["LARGE_RADIUS"] if axis == 0 else G["SMALL_RADIUS"]
    circle = G["complete_circle"](G["right_semicircle"](radius, 32))

    def point(axial, u, v):
        p = [0., 0., 0.]
        p[axis], p[j], p[k] = axial, u, v
        return tuple(p)

    for sign in (-1, 1):
        mouth = [point(sign * half[axis], u, v) for u, v in circle]
        bottom = [point(sign * (half[axis] - 1.25), u, v) for u, v in circle]
        apex = point(sign * (half[axis] - 2.5), 0., 0.)
        for index, (a, b) in enumerate(G["pairs"](mouth)):
            c, d = bottom[index], bottom[(index + 1) % len(bottom)]
            inward = point(0., -a[j] - b[j], -a[k] - b[k])
            mesh.quad(a, b, d, c, inward, "blind_wall")
            mesh.triangle(c, d, apex, inward, "conical_tip")
        G["planar_face"](mesh, half, axis, sign, mouth)
for sign in (-1, 1):
    G["planar_face"](mesh, half, 1, sign)

path = ROOT / "fixtures/historical_four_blind_drills.stl"
G["write_stl"](path, mesh.faces, False)
faces = G["read_stl"](path)
edges = defaultdict(list)
vertices = set()
neighbors = {i: set() for i in range(len(faces))}
for i, face in enumerate(faces):
    a, b, c = face
    assert G["cross"](G["sub"](b, a), G["sub"](c, a)) != (0., 0., 0.)
    vertices.update(face)
    for a, b in G["pairs"](face):
        edges[tuple(sorted((a, b)))].append((a, b, i))
for (a, b, i), (d, c, j) in edges.values():
    assert (a, b) == (c, d)
    neighbors[i].add(j)
    neighbors[j].add(i)
assert len(G["reachable"](neighbors)) == len(faces)
assert len(vertices) - len(edges) + len(faces) == 2
for point, material in [
    ((0., 0., 0.), True),
    ((-11., 0., 0.), False), ((11., 0., 0.), False),
    ((0., 0., -8.), False), ((0., 0., 8.), False),
]:
    assert abs(G["winding"](faces, point) - float(material)) < 1e-7
print(path.name, len(faces), "facets, connected and closed, chi = 2")
