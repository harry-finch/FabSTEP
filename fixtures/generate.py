"""Independent, fixed analytical fixtures. Run from any directory with Python 3.

No converter code is imported. Winding is derived from explicitly supplied
counterclockwise profiles and cap triangulations. Dimensions are millimetres.
"""
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def write(name, vertices, faces):
    lines = [f"solid {name}"]
    for face in faces:
        # The converter must calculate normals rather than trust stored normals.
        lines += ["  facet normal 0 0 0", "    outer loop"]
        lines += ["      vertex " + " ".join(str(c) for c in vertices[i]) for i in face]
        lines += ["    endloop", "  endfacet"]
    lines += [f"endsolid {name}", ""]
    (ROOT / f"{name}.stl").write_text("\n".join(lines))


def prism(name, profile, cap, height, holes=()):
    count = len(profile)
    vertices = [(x, y, z) for z in (0, height) for x, y in profile]
    faces = [(c, b, a) for a, b, c in cap]
    faces += [(a + count, b + count, c + count) for a, b, c in cap]
    loops = [list(range(count))] if not holes else [list(range(4))] + list(holes)
    for loop in loops:
        for a, b in zip(loop, loop[1:] + loop[:1]):
            faces += [(a, b, b + count), (a, b + count, a + count)]
    write(name, vertices, faces)


prism("cube", [(0, 0), (1, 0), (1, 1), (0, 1)], [(0, 1, 2), (0, 2, 3)], 1)
prism("concave_prism", [(0, 0), (2, 0), (2, 1), (1, 1), (1, 2), (0, 2)],
      [(0, 1, 3), (1, 2, 3), (0, 3, 5), (3, 4, 5)], 2)
profile = [(0, 0), (4, 0), (4, 4), (0, 4), (1, 1), (3, 1), (3, 3), (1, 3)]
cap = []
for i in range(4):
    j = (i + 1) % 4
    cap += [(i, j, j + 4), (i, j + 4, i + 4)]
prism("rectangular_hole", profile, cap, 2, holes=([7, 6, 5, 4],))
