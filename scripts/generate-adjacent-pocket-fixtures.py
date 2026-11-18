#!/usr/bin/env python3
"""Generate two concave pocket fillets meeting on a measured elliptic seam."""
import argparse
from collections import defaultdict
from dataclasses import replace
import hashlib
import json
import math
from pathlib import Path
import runpy

BASE = runpy.run_path(
    str(Path(__file__).with_name("generate-pocket-fillet-fixtures.py"))
)
OUT = Path(__file__).resolve().parents[1] / "fixtures" / "adjacent_pocket"


def ring(x0, x1, y0, y1, z, subdivisions):
    corners = [(x0, y0, z), (x1, y0, z), (x1, y1, z), (x0, y1, z)]
    points = []
    for i, a in enumerate(corners):
        b = corners[(i + 1) % 4]
        points.append(a)
        for k in range(1, subdivisions):
            points.append(tuple(x + (y - x) * k / subdivisions for x, y in zip(a, b)))
    return points


def write(out, name, d, n, bands, rotate=False, warped=False):
    length, width, radius = d.pocket_length, d.pocket_width, d.radius
    x0, y0, z0 = -length / 2, -width / 2, d.floor
    cx, cy, cz = x0 + radius, y0 + radius, z0 + radius
    groups = defaultdict(list)

    def quad(group, a, b, c, e, alternate):
        groups[group].extend(
            [(a, b, e), (b, c, e)] if alternate else [(a, b, c), (a, c, e)]
        )

    def place(p):
        x, y, z = p
        return (z + 6, x - 4, y + 2) if rotate else p

    outer_base = ring(-d.length / 2, d.length / 2, -d.width / 2, d.width / 2, 0, bands)
    outer_top = [(x, y, d.height) for x, y, _ in outer_base]
    mouth = ring(x0, -x0, y0, -y0, d.height, bands)
    count = len(mouth)
    for i in range(count):
        j = (i + 1) % count
        groups["outer_bottom"].append(((0, 0, 0), outer_base[j], outer_base[i]))
        side = ("outer_y_min", "outer_x_max", "outer_y_max", "outer_x_min")[i // bands]
        quad(side, outer_base[i], outer_base[j], outer_top[j], outer_top[i], i % 2)
        quad("rim", outer_top[i], outer_top[j], mouth[j], mouth[i], i % 2)

    # Above the floor the removed cross-section is a rectangle whose two
    # minimum coordinates move together. Its lower-left vertex traces the
    # exact ellipse shared by the two cylindrical walls.
    rows = [ring(cx, -x0, cy, -y0, z0, bands)]
    for k in range(1, n):
        t = math.pi * k / (2 * n)
        q = radius * (1 - math.sin(t))
        z = cz - radius * math.cos(t)
        rows.append(ring(x0 + q, -x0, y0 + q, -y0, z, bands))
    rows.append(ring(x0, -x0, y0, -y0, cz, bands))
    rows.append(mouth)
    for k, (bottom, top) in enumerate(zip(rows, rows[1:])):
        groups_here = (
            "fillet_x" if k < n else "wall_y_min",
            "stop_x_max",
            "stop_y_max",
            "fillet_y" if k < n else "wall_x_min",
        )
        for i in range(count):
            j = (i + 1) % count
            quad(
                groups_here[i // bands],
                bottom[i],
                top[i],
                top[j],
                bottom[j],
                (k + i) % 2,
            )
    center = ((cx - x0) / 2, (cy - y0) / 2, z0)
    for i in range(count):
        groups["floor"].append((center, rows[0][i], rows[0][(i + 1) % count]))

    damaged = rows[n // 2][0]

    def vertex(p):
        if warped and p == damaged:
            # Leave the seam in its diagonal plane but move one interior knot
            # off both cylinders. All incident triangles share that knot.
            return place((p[0] + 0.14, p[1] + 0.14, p[2]))
        return place(p)

    faces, ranges = [], {}
    for group, triangles in groups.items():
        start = len(faces)
        faces.extend(tuple(vertex(v) for v in tri) for tri in triangles)
        ranges[group] = [start, len(faces)]
    data, triangles = BASE["binary_mesh"](faces)
    mesh_volume, boundary = BASE["mesh_measurements"](triangles)
    assert boundary == 0 and mesh_volume > 0

    def group_edges(group):
        start, end = ranges[group]
        return {
            (p, q)
            for a, b, c in triangles[start:end]
            for p, q in ((a, b), (b, c), (c, a))
        }

    shared = {(b, a) for a, b in group_edges("fillet_x")} & group_edges("fillet_y")
    assert len(shared) == n
    assert len({v for edge in shared for v in edge}) == n + 1
    (out / (name + ".stl")).write_bytes(data)
    case = dict(
        triangles=len(faces),
        groups=ranges,
        seam_segments=len(shared),
        mesh_volume_mm3=mesh_volume,
        sha256=hashlib.sha256(data).hexdigest(),
    )
    if warped:
        case.update(warped_knot_mm=place(damaged), displaced_knot_mm=vertex(damaged))
        return case

    # At level z the lost void area is (length+width)*q(z)-q(z)^2,
    # q(z)=r-sqrt(r²-(cz-z)²). Integrating from z0 to cz gives:
    volume = (
        d.length * d.width * d.height
        - length * width * (d.height - z0)
        + (length + width) * radius**2 * (1 - math.pi / 4)
        - radius**3 * (5 / 3 - math.pi / 2)
    )
    # Linear chord interpolation increases q. The derivative of lost void
    # area with respect to q is at most length+width.
    chord_area = radius**2 / 2 * (math.pi / 2 - n * math.sin(math.pi / (2 * n)))
    assert 0 < mesh_volume - volume < (length + width) * chord_area + 0.001
    corners = [
        place((x, y, z))
        for x in (-d.length / 2, d.length / 2)
        for y in (-d.width / 2, d.width / 2)
        for z in (0, d.height)
    ]
    case.update(
        radius_mm=radius,
        volume_mm3=volume,
        bounds_mm=[[op(p[i] for p in corners) for i in range(3)] for op in (min, max)],
        seam_center_mm=place((cx, cy, cz)),
        seam_major_radius_mm=math.sqrt(2) * radius,
        seam_minor_radius_mm=radius,
        seam_endpoints_mm=[place((x0, y0, cz)), place((cx, cy, z0))],
        seam_midpoint_mm=place(
            (
                cx - radius / math.sqrt(2),
                cy - radius / math.sqrt(2),
                cz - radius / math.sqrt(2),
            )
        ),
        stop_circles={
            axis: dict(
                center_mm=place(c),
                radius_mm=radius,
                endpoints_mm=[place(start), place(end)],
            )
            for axis, c, start, end in [
                ("x", (-x0, cy, cz), (-x0, y0, cz), (-x0, cy, z0)),
                ("y", (cx, -y0, cz), (x0, -y0, cz), (cx, -y0, z0)),
            ]
        },
        inside_void_mm=place((0, 0, (d.height + z0) / 2)),
        inside_material_mm=place(
            (cx - 0.85 * radius, cy - 0.85 * radius, cz - 0.85 * radius)
        ),
    )
    return case


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, nargs="?", default=OUT)
    out = parser.parse_args().output
    out.mkdir(parents=True, exist_ok=True)
    d = BASE["Dimensions"]()
    cases = {
        "nominal": write(out, "nominal", d, 16, 2),
        "rotated": write(
            out,
            "rotated",
            replace(d, pocket_length=18, pocket_width=12, radius=3),
            23,
            3,
            rotate=True,
        ),
        "warped_seam": write(out, "warped_seam", d, 16, 2, warped=True),
    }
    manifest = dict(
        unit="mm",
        positives=["nominal", "rotated"],
        negatives=["warped_seam"],
        planes=11,
        cylinders=2,
        cases=cases,
    )
    (out / "manifest.json").write_text(
        json.dumps(manifest, indent=2, allow_nan=False) + "\n"
    )


if __name__ == "__main__":
    main()
