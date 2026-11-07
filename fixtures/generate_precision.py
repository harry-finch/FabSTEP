"""Synthetic quantization corpus; no private/user geometry is read.

The base is the existing independent Python M2c arc + hole + boss fixture.
Only numeric serialization changes in the rounded case. Topology, units and
placements stay identical. Six significant digits model an exporter precision
loss, not a property promised by the 3MF format.
"""
import json
import math
from pathlib import Path
import generate_m3

ROOT = Path(__file__).resolve().parent
SOURCE = "m2c_open_arc_mixed.stl"


def read_mesh():
    vertices, triangles, indices = [], [], {}
    triangle = []
    for line in (ROOT / SOURCE).read_text().splitlines():
        words = line.split()
        if words[:1] != ["vertex"]:
            continue
        point = tuple(map(float, words[1:]))
        if point not in indices:
            indices[point] = len(vertices)
            vertices.append(point)
        triangle.append(indices[point])
        if len(triangle) == 3:
            triangles.append(tuple(triangle))
            triangle = []
    assert not triangle
    return vertices, triangles


def write_stl(name, vertices, triangles):
    lines = ["solid " + name]
    for triangle in triangles:
        lines.extend([" facet normal 0 0 0", "  outer loop"])
        lines.extend("   vertex " + " ".join(format(v, ".17g") for v in vertices[i]) for i in triangle)
        lines.extend(["  endloop", " endfacet"])
    (ROOT / (name + ".stl")).write_text("\n".join(lines + ["endsolid " + name]) + "\n")


def main():
    exact, triangles = read_mesh()
    rounded = [tuple(float(format(v, ".6g")) for v in p) for p in exact]
    assert len(set(rounded)) == len(exact), "Rounding must not merge vertices"
    noisy = list(rounded)
    # A real top-rim vertex of the synthetic boss, changed only in this negative case.
    index = noisy.index((50.0, 45.0, 23.0))
    noisy[index] = (50.002, 45.0, 23.0)
    for name, vertices in [("precision_exact", exact), ("precision_rounded", rounded), ("precision_outlier", noisy)]:
        xml = generate_m3.model([generate_m3.mesh_object(1, vertices, triangles, name)], [(1, None)])
        generate_m3.write(name, xml)
    write_stl("precision_rounded", rounded, triangles)
    manifest = {
        "unit": "mm", "synthetic_source": SOURCE,
        "generator": "Python standard library; indexed mesh extracted from synthetic M2c fixture",
        "triangles": len(triangles), "vertices": len(exact),
        "serialization": "six significant decimal digits, same indexed topology",
        "max_vertex_displacement_mm": max(math.dist(a, b) for a, b in zip(exact, rounded)),
        "negative_outlier": {"vertex": index, "original_mm": rounded[index], "changed_mm": noisy[index]},
        "expectations_to_verify": {
            "precision_exact": {"default_fit_mm": 0.00001, "cylinders": 3, "planes": 7},
            "precision_rounded": {"strict_refusal": True, "explicit_fit_mm": 0.0001, "cylinders": 3, "planes": 7},
            "precision_outlier": {"refused_with_explicit_fit_mm": 0.0001},
            "deviation_budget": {"rounded_refused_below_chord_mm": 0.001},
        },
    }
    (ROOT / "precision_manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
