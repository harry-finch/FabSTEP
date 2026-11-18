#!/usr/bin/env python3
"""Cut a cylindrical pocket at a measured plane and generate analytic references."""

import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path
import runpy
import struct


BASE = runpy.run_path(
    str(Path(__file__).with_name("generate-pocket-fillet-fixtures.py"))
)
OUT = Path(__file__).resolve().parents[1] / "fixtures" / "oblique_pocket"


def write(
    out, name, sample, axial_intervals, intervals, slope_y, slope_z, rotate=False
):
    d = BASE["Dimensions"]()
    facets, groups = BASE["generate"](sample, d, axial_intervals, intervals)
    radius = d.radius
    cy, cz = -d.pocket_width / 2 + radius, d.floor + radius
    half = d.pocket_length / 2

    def place(p):
        return BASE["place"](p, "rotated") if rotate else p

    def direction(v):
        return [b - a for a, b in zip(place((0, 0, 0)), place(v))]

    def end(y, z):
        return half + slope_y * y + slope_z * (z - cz)

    def shape(p):
        x, y, z = p
        if abs(x) <= half and abs(y) <= d.pocket_width / 2 and z >= d.floor:
            fraction = (x + half) / d.pocket_length
            x += fraction * (end(y, z) - half)
        return place((x, y, z))

    triangles = [tuple(shape(v) for v in tri) for tri in facets]
    if rotate:
        lines = ["solid oblique_pocket"]
        for tri in triangles:
            lines.extend(["facet normal 0 0 0", "outer loop"])
            lines.extend("vertex " + " ".join(f"{x:.17g}" for x in v) for v in tri)
            lines.extend(["endloop", "endfacet"])
        data = ("\n".join(lines + ["endsolid oblique_pocket"]) + "\n").encode()
    else:
        facet = BASE["FACET"]
        data = bytearray(80) + struct.pack("<I", len(triangles))
        for tri in triangles:
            data.extend(facet.pack(0, 0, 0, *(x for v in tri for x in v), 0))
        triangles = [(v[3:6], v[6:9], v[9:12]) for v in facet.iter_unpack(data[84:])]
    (out / f"{name}.stl").write_bytes(data)
    edges = Counter()
    for a, b, c in triangles:
        assert len({a, b, c}) == 3
        edges.update(((a, b), (b, c), (c, a)))
    assert all(n == 1 and edges[b, a] == 1 for (a, b), n in edges.items())
    mesh_volume = math.fsum(BASE["signed_tetrahedron"](*tri) for tri in triangles)
    case = dict(
        triangles=len(triangles),
        groups=groups,
        mesh_volume_mm3=mesh_volume,
        sha256=hashlib.sha256(data).hexdigest(),
    )
    if sample == "warped_stop":
        return case

    # Integrate the local pocket length over the sharp rectangle, then restore
    # the material outside the quarter-disk at the rounded lower corner.
    area = d.pocket_width * (d.height - d.floor)
    mean_length = d.pocket_length + slope_z * ((d.height + d.floor) / 2 - cz)
    retained_area = radius**2 * (1 - math.pi / 4)
    volume = (
        d.length * d.width * d.height
        - area * mean_length
        + (end(cy, cz) + half) * retained_area
        - (slope_y + slope_z) * radius**3 / 6
    )
    max_length = max(
        end(y, z) + half
        for y in (-d.pocket_width / 2, d.pocket_width / 2)
        for z in (d.floor, d.height)
    )
    # All cylinder triangles span at most one of the coarsest angular
    # intervals; the circular annular sector bounds their volume error.
    shell_volume = (
        max_length * math.pi * radius**2 / 4 * math.sin(math.pi / (4 * intervals)) ** 2
    )
    assert 0 < mesh_volume - volume < shell_volume + 0.001, (name, mesh_volume, volume)

    g = math.hypot(slope_y, slope_z)
    stretch = math.hypot(1, g)
    major = (g / stretch, slope_y / g / stretch, slope_z / g / stretch)
    minor = (0.0, -slope_z / g, slope_y / g)

    def on_end(y, z):
        return place((end(y, z), y, z))

    corners = [
        place((x, y, z))
        for x in (-d.length / 2, d.length / 2)
        for y in (-d.width / 2, d.width / 2)
        for z in (0, d.height)
    ]
    case.update(
        radius_mm=radius,
        volume_mm3=volume,
        axis=direction((1, 0, 0)),
        bounds_mm=[[f(v[i] for v in corners) for i in range(3)] for f in (min, max)],
        circular_center_mm=place((-half, cy, cz)),
        ellipse_center_mm=on_end(cy, cz),
        ellipse_major_radius_mm=radius * stretch,
        ellipse_minor_radius_mm=radius,
        ellipse_major_direction=direction(major),
        ellipse_minor_direction=direction(minor),
        ellipse_endpoints_mm=[on_end(cy - radius, cz), on_end(cy, cz - radius)],
        ellipse_midpoint_mm=on_end(
            cy - radius / math.sqrt(2), cz - radius / math.sqrt(2)
        ),
        stop_max_normal=direction(
            (1 / stretch, -slope_y / stretch, -slope_z / stretch)
        ),
        floor_normal=direction((0, 0, 1)),
        tangent_wall_normal=direction((0, 1, 0)),
        before_oblique_stop_mm=place((end(-2, 8) - 0.15, -2, 8)),
        beyond_oblique_stop_mm=place((end(-2, 8) + 0.15, -2, 8)),
        inside_void_mm=place((0, 0, (d.floor + d.height) / 2)),
        inside_material_mm=place((0, cy - 0.85 * radius, cz - 0.85 * radius)),
    )
    return case


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, nargs="?", default=OUT)
    out = parser.parse_args().output
    out.mkdir(parents=True, exist_ok=True)
    cases = {
        name: write(out, name, *params)
        for name, *params in [
            ("nominal", "nominal", 2, 16, 0.5, 0),
            ("reverse", "retessellated", 3, 20, -0.5, 0),
            ("staggered", "staggered", 1, 16, 0.5, 0),
            ("rotated", "nominal", 2, 16, 0.5, 0, True),
            ("compound", "retessellated", 3, 23, 0.5, -0.25),
            ("warped_stop", "warped_stop", 1, 16, 0.5, 0),
        ]
    }
    manifest = dict(
        unit="mm",
        positives=[name for name in cases if name != "warped_stop"],
        negatives=["warped_stop"],
        planes=11,
        cylinders=1,
        cases=cases,
    )
    (out / "manifest.json").write_text(
        json.dumps(manifest, indent=2, allow_nan=False) + "\n"
    )


if __name__ == "__main__":
    main()
