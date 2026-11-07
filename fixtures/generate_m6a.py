"""Generate the FreeCAD rounded enclosure and independent coarse patch meshes."""

from itertools import product
import json
import math
from pathlib import Path

import generate_m3
from generate_m2b import transformed
from generate_m5d import write_stl


ROOT = Path(__file__).resolve().parent
HALF = (12.0, 8.0, 5.0)
CENTER = (14.0, 10.0, 7.0)
RADIUS = 2.0
AXES = ((1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0))
DENSE_DEFLECTION = 0.01


def patch_mesh(divisions=8):
    """Sample planes, quarter cylinders and spherical octants independently."""
    vertices = []
    triangles = []
    indexed = {}

    def point(local):
        key = tuple(round(value, 12) for value in local)
        if key not in indexed:
            indexed[key] = len(vertices)
            vertices.append(tuple(value + center for value, center in zip(key, CENTER)))
        return indexed[key]

    def triangle(a, b, c):
        p, q, r = (vertices[i] for i in (a, b, c))
        u, v = [x - y for x, y in zip(q, p)], [x - y for x, y in zip(r, p)]
        normal = [u[(i + 1) % 3] * v[(i + 2) % 3] - u[(i + 2) % 3] * v[(i + 1) % 3] for i in range(3)]
        mid = [(x + y + z) / 3.0 - center for x, y, z, center in zip(p, q, r, CENTER)]
        outward = [v - max(-h, min(h, v)) for v, h in zip(mid, HALF)]
        dot = sum(a * b for a, b in zip(normal, outward))
        assert dot != 0.0
        triangles.append((a, b, c) if dot > 0.0 else (a, c, b))

    def quad(a, b, c, d):
        triangle(a, b, c)
        triangle(a, c, d)

    angles = [math.pi / 2.0 * i / divisions for i in range(divisions + 1)]
    for axis in range(3):
        j, k = (axis + 1) % 3, (axis + 2) % 3
        for sign in (-1, 1):
            corners = []
            for sj, sk in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
                p = [0.0] * 3
                p[axis], p[j], p[k] = sign * (HALF[axis] + RADIUS), sj * HALF[j], sk * HALF[k]
                corners.append(point(p))
            quad(*corners)
        for sj, sk in product((-1, 1), repeat=2):
            stations = []
            for theta in angles:
                p = [0.0] * 3
                p[j] = sj * (HALF[j] + RADIUS * math.cos(theta))
                p[k] = sk * (HALF[k] + RADIUS * math.sin(theta))
                stations.append([])
                for sign in (-1, 1):
                    p[axis] = sign * HALF[axis]
                    stations[-1].append(point(p))
            for (a, b), (d, c) in zip(stations, stations[1:]):
                quad(a, b, c, d)
    for signs in product((-1, 1), repeat=3):
        rings = []
        for v in angles[:-1]:
            rings.append([])
            for u in angles:
                radial = (math.cos(v) * math.cos(u), math.cos(v) * math.sin(u), math.sin(v))
                rings[-1].append(point([s * (h + RADIUS * n) for s, h, n in zip(signs, HALF, radial)]))
        pole = point((signs[0] * HALF[0], signs[1] * HALF[1], signs[2] * (HALF[2] + RADIUS)))
        for lower, upper in zip(rings, rings[1:]):
            for i in range(divisions):
                quad(lower[i], lower[i + 1], upper[i + 1], upper[i])
        for i in range(divisions):
            triangle(rings[-1][i], rings[-1][i + 1], pole)
    return vertices, triangles


def box_volume(half, r):
    x, y, z = [2.0 * h for h in half]
    return x * y * z + 2 * r * (x * y + x * z + y * z) + math.pi * r**2 * (x + y + z) + 4 * math.pi * r**3 / 3


def reference(triangles, center=CENTER, axes=AXES):
    r = RADIUS
    extent = [r + sum(h * abs(axis[i]) for h, axis in zip(HALF, axes)) for i in range(3)]
    return {
        "triangles": triangles,
        "faces": 26, "planes": 6, "cylinders": 12, "spheres": 8,
        "center_mm": center, "axes": axes, "core_half_extents": HALF, "radius_mm": r,
        "bounds_mm": [[c + sign * d for c, d in zip(center, extent)] for sign in (-1, 1)],
        "volume_mm3": box_volume(HALF, r),
    }


def main():
    import FreeCAD
    from complex_v1.freecad_corpus import rounded_enclosure

    cases = {}
    dense = rounded_enclosure().removeSplitter()
    points, faces = dense.tessellate(DENSE_DEFLECTION)
    assert len(faces) == 28_700, "Le témoin dense doit reproduire le cas 04."
    write_stl("m6a_rounded_box", [(p.x, p.y, p.z) for p in points], faces)
    cases["m6a_rounded_box"] = {
        **reference(len(faces)),
        "generator": "FreeCAD " + ".".join(FreeCAD.Version()[:3]),
        "tessellation_deflection_mm": DENSE_DEFLECTION,
    }

    points, faces = patch_mesh()
    origin = transformed((0.0, 0.0, 0.0))
    axes = tuple(tuple(x - y for x, y in zip(transformed(axis), origin)) for axis in AXES)
    write_stl("m6a_rounded_box_rotated", [transformed(p) for p in points], faces)
    rotated = reference(len(faces), transformed(CENTER), axes)
    # Maximum coordinate rounding of the binary STL inside its nominal box.
    # A normal binary32 value has 24 significant bits, including the leading one.
    half_ulps = [math.ldexp(1.0, math.frexp(max(map(abs, interval)))[1] - 25)
                 for interval in zip(*rotated["bounds_mm"])]
    rotated["source_coordinate_quantization_bound_mm"] = math.hypot(*half_ulps)
    cases["m6a_rounded_box_rotated"] = rotated

    resource = generate_m3.mesh_object(1, points, faces, "Boîtier R2")
    mirror = "-1 0 0 0 1 0 0 0 1 63 3 2"
    generate_m3.write("m6a_rounded_box_instances", generate_m3.model([resource], [(1, None), (1, mirror)]))
    cases["m6a_rounded_box_instances"] = {
        "format": "3mf", "triangles": 2 * len(faces),
        "bodies": [reference(len(faces)), reference(len(faces), (49.0, 13.0, 9.0))],
    }
    (ROOT / "m6a_manifest.json").write_text(
        json.dumps({"unit": "mm", "cases": cases}, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
