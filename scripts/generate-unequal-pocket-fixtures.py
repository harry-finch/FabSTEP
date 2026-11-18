#!/usr/bin/env python3
"""Generate closed blind pockets with two different adjacent constant fillet radii."""
import argparse
from dataclasses import replace
import hashlib
import json
import math
from pathlib import Path
import runpy

ADJACENT = runpy.run_path(
    str(Path(__file__).with_name("generate-adjacent-pocket-fixtures.py"))
)
BASE = ADJACENT["BASE"]
OUT = Path(__file__).resolve().parents[1] / "fixtures" / "unequal_pocket"


def inset(radius, height):
    """Distance removed from the rectangular opening at height above the floor."""
    if height >= radius:
        return 0.0
    return radius - math.sqrt(height * (2 * radius - height))


def material_volume(d, rx, ry):
    """Horizontal slices, with rx the radius of the cylinder parallel to X."""
    s, large = sorted((rx, ry))
    delta, total = large - s, large + s

    def circular_integral(r):
        # Integral from 0 to s of sqrt(h*(2*r-h)).
        return ((s - r) * math.sqrt(s * (2 * r - s)) + r * r * math.acos(1 - s / r)) / 2

    def primitive(u):
        root = math.sqrt(u * u - delta * delta)
        return (
            total * (u * root - delta * delta * math.acosh(u / delta)) / 2 - root**3 / 3
        )

    # Product of the two circular insets only overlaps up to the smaller radius.
    overlap = (
        rx * ry * s
        - rx * circular_integral(ry)
        - ry * circular_integral(rx)
        + primitive(total)
        - primitive(total - s)
    )
    retained = (d.pocket_length * rx**2 + d.pocket_width * ry**2) * (
        1 - math.pi / 4
    ) - overlap
    # Independent numerical check of the overlap. h = s*(1-cos(t)) removes the
    # square-root singularity at the lower integration endpoint.
    bins = 2048
    dt = math.pi / (2 * bins)
    values = []
    for i in range(bins + 1):
        t = i * dt
        h = s * (1 - math.cos(t))
        f = inset(rx, h) * inset(ry, h) * s * math.sin(t)
        values.append(f * (1 if i in (0, bins) else 4 if i % 2 else 2))
    assert abs(overlap - dt * math.fsum(values) / 3) < 1e-9
    return (
        d.length * d.width * d.height
        - d.pocket_length * d.pocket_width * (d.height - d.floor)
        + retained
    )


def write(out, name, d, rx, ry, n, bands, reflected=False, warped=False):
    small, large = sorted((rx, ry))
    assert (
        0 < small < large < d.height - d.floor
        and rx < d.pocket_width
        and ry < d.pocket_length
    )
    x0, y0 = -d.pocket_length / 2, -d.pocket_width / 2

    def place(p):
        x, y, z = p
        return (6 - z, x - 4, y + 2) if reflected else p

    def corner(h):
        return x0 + inset(ry, h), y0 + inset(rx, h), d.floor + h

    # Split exactly at the tangency level of the smaller cylinder. Above this
    # level the bigger fillet terminates directly on a planar wall.
    heights = [small * (1 - math.cos(math.pi * k / (2 * n))) for k in range(n)] + [
        small
    ]
    start_angle = math.acos(1 - small / large)
    quarter = math.pi / 2
    extra = max(2, math.ceil(n * (quarter - start_angle) / quarter))
    heights.extend(
        large * (1 - math.cos(start_angle + (quarter - start_angle) * k / extra))
        for k in range(1, extra)
    )
    heights.extend((large, d.height - d.floor))
    rows = []
    for h in heights:
        x, y, z = corner(h)
        rows.append(ADJACENT["ring"](x, -x0, y, -y0, z, bands))
    groups = ADJACENT["pocket_shell"](
        d, bands, rows, [(h < rx, h < ry) for h in heights[:-1]]
    )
    damaged = rows[n // 2][0]

    def vertex(p):
        if warped and p == damaged:
            p = p[0] + 0.14, p[1] + 0.14, p[2]
        return place(p)

    triangles, ranges = [], {}
    for group, faces in groups.items():
        first = len(triangles)
        for face in faces:
            triangles.append(
                tuple(vertex(p) for p in (face[::-1] if reflected else face))
            )
        ranges[group] = [first, len(triangles)]
    data, triangles = BASE["binary_mesh"](triangles)
    mesh_volume, boundary = BASE["mesh_measurements"](triangles)
    assert boundary == 0 and mesh_volume > 0

    def directed(group):
        start, end = ranges[group]
        return {
            (p, q)
            for a, b, c in triangles[start:end]
            for p, q in ((a, b), (b, c), (c, a))
        }

    def shared_segments(a, b):
        return len({(q, p) for p, q in directed(a)} & directed(b))

    large_wall = "fillet_y" if ry > rx else "fillet_x"
    small_side = "wall_y_min" if ry > rx else "wall_x_min"
    assert shared_segments("fillet_x", "fillet_y") == n
    assert shared_segments(large_wall, small_side) == extra
    (out / (name + ".stl")).write_bytes(data)
    case = dict(
        triangles=len(triangles),
        groups=ranges,
        sha256=hashlib.sha256(data).hexdigest(),
        mesh_volume_mm3=mesh_volume,
        seam_segments=n,
        continuation_segments=extra,
    )
    if warped:
        case.update(original_knot_mm=place(damaged), displaced_knot_mm=vertex(damaged))
        return case

    volume = material_volume(d, rx, ry)
    # Along each side the polygonal section deviates by the sum of the
    # circular segments. The opposing span bounds the omitted strip of void.
    excess_bound = 0
    for r, span in ((rx, d.pocket_length), (ry, d.pocket_width)):
        angles = [math.acos(1 - h / r) for h in heights if h <= r]
        excess_bound += (
            span
            * r
            * r
            / 2
            * math.fsum((b - a) - math.sin(b - a) for a, b in zip(angles, angles[1:]))
        )
    assert 0 < mesh_volume - volume < excess_bound + 0.001
    branch = [place(corner(small * t)) for t in (0.0, 0.125, 0.375, 0.75, 1.0)]

    # The fourth sample is outside the plane through the first, third and last.
    def vector(p):
        return tuple(b - a for a, b in zip(branch[0], p))

    v, w = vector(branch[2]), vector(branch[4])
    normal = (
        v[1] * w[2] - v[2] * w[1],
        v[2] * w[0] - v[0] * w[2],
        v[0] * w[1] - v[1] * w[0],
    )
    distance = abs(sum(a * b for a, b in zip(normal, vector(branch[3])))) / math.hypot(
        *normal
    )
    assert distance > 1e-4
    case.update(
        radius_by_local_axis_mm=dict(x=rx, y=ry),
        volume_mm3=volume,
        bounds_mm=[
            [
                op(
                    place((x, y, z))[i]
                    for x in (-d.length / 2, d.length / 2)
                    for y in (-d.width / 2, d.width / 2)
                    for z in (0, d.height)
                )
                for i in range(3)
            ]
            for op in (min, max)
        ],
        shared_branch_mm=branch,
        fourth_sample_distance_from_plane_mm=distance,
        continuation=dict(
            cylinder=large_wall,
            plane=small_side,
            start_mm=place(corner(small)),
            end_mm=place(corner(large)),
            sweep_rad=quarter - start_angle,
        ),
        circular_stops=dict(
            x=dict(center_mm=place((-x0, y0 + rx, d.floor + rx)), plane="stop_x_max"),
            y=dict(center_mm=place((x0 + ry, -y0, d.floor + ry)), plane="stop_y_max"),
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
        "nominal": write(out, "nominal", d, 2.0, 3.0, 18, 2),
        "reflected": write(
            out,
            "reflected",
            replace(d, pocket_length=18, pocket_width=12),
            3.5,
            1.5,
            24,
            3,
            reflected=True,
        ),
        "warped_seam": write(out, "warped_seam", d, 2.0, 3.0, 18, 2, warped=True),
    }
    manifest = dict(
        unit="mm",
        positives=["nominal", "reflected"],
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
