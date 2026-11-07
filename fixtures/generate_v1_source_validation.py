"""Independent regression: planar grouping must never hide source intersections.

A 1000 x 1000 x 0.001 mm plate has a horizontal perimeter and an interior
four-triangle depression. Negative center Z crosses the bottom face; positive
center Z is a valid shallow depression. Both group into six planes at 0.01 mm,
so validity of the grouped shape alone cannot distinguish them.
"""
from pathlib import Path
import generate_m3

ROOT = Path(__file__).resolve().parent


def plate(center_z):
    points = [
        (0, 0, 0), (1000, 0, 0), (1000, 1000, 0), (0, 1000, 0),
        (0, 0, .001), (1000, 0, .001), (1000, 1000, .001), (0, 1000, .001),
        (250, 250, .001), (750, 250, .001), (750, 750, .001), (250, 750, .001),
        (500, 500, center_z),
    ]
    triangles = [(0, 2, 1), (0, 3, 2)]
    for i in range(4):
        j = (i + 1) % 4
        triangles += [(4+i, 4+j, 8+j), (4+i, 8+j, 8+i), (8+i, 8+j, 12)]
    triangles += [(0, 1, 5), (0, 5, 4), (1, 2, 6), (1, 6, 5),
                  (2, 3, 7), (2, 7, 6), (3, 0, 4), (3, 4, 7)]
    return points, triangles


def main():
    for name, center_z in [("v1_intersecting_plate", -.001), ("v1_shallow_plate", .0005)]:
        points, triangles = plate(center_z)
        lines = ["solid " + name]
        for triangle in triangles:
            lines += ["facet normal 0 0 0", "outer loop"]
            lines += ["vertex " + " ".join(map(str, points[i])) for i in triangle]
            lines += ["endloop", "endfacet"]
        lines += ["endsolid " + name]
        (ROOT / (name + ".stl")).write_text("\n".join(lines) + "\n")
        if center_z < 0:
            obj = generate_m3.mesh_object(1, points, triangles, "Plaque auto-intersectante")
            generate_m3.write(name, generate_m3.model([obj], [(1, None)]))


if __name__ == "__main__":
    main()
