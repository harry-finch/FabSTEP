#!/usr/bin/env python3
"""Independent D1/E1 nominal surfaces; no FabSTEP or OCCT dependency.

D1 rounds one or two disjoint vertical edges. E1 has eight vertical circular
corners and an actual floor and annular rim. Floor/rim joins are deliberately
sharp. Each source is a closed material boundary, including the open tray.
"""

import hashlib
import json
import math
from pathlib import Path


OUT = Path(__file__).parent / "partial_fillets"
OUT.mkdir(exist_ok=True)
WIDTH, DEPTH, HEIGHT = 40.0, 30.0, 12.0
FLOOR = 3.0
SEGMENTS = 16


def profile(width, depth, radii, *, stagger=False):
    """CCW, from bottom-right: BR, TR, TL, BL radii, independently measured."""
    points = []
    for corner, (sx, sy) in enumerate(((1, -1), (1, 1), (-1, 1), (-1, -1))):
        radius = radii[corner]
        if radius == 0:
            points.append((sx * width / 2, sy * depth / 2))
            continue
        center = (sx * (width / 2 - radius), sy * (depth / 2 - radius))
        start = (corner - 1) * math.pi / 2
        for i in range(SEGMENTS + 1):
            fraction = i / SEGMENTS
            angle = start + fraction * math.pi / 2
            if stagger and 0 < i < SEGMENTS:
                angle += 0.24 * math.pi / (2 * SEGMENTS) * math.sin(math.pi * fraction)
            points.append(tuple(center[j] + radius * f(angle)
                                for j, f in enumerate((math.cos, math.sin))))
    return points


def wall(triangles, lower, upper, *, inner=False):
    assert len(lower) == len(upper)
    for i in range(len(lower)):
        j = (i + 1) % len(lower)
        pair = [[lower[i], lower[j], upper[j]], [lower[i], upper[j], upper[i]]]
        triangles.extend([list(reversed(t)) for t in pair] if inner else pair)


def cap(triangles, ring, *, downward=False):
    center = (0.0, 0.0, ring[0][2])
    for i in range(len(ring)):
        tri = [center, ring[i], ring[(i + 1) % len(ring)]]
        triangles.append(list(reversed(tri)) if downward else tri)


def ring(width, depth, radii, z, **kwargs):
    return [(x, y, z) for x, y in profile(width, depth, radii, **kwargs)]


def box(radii, *, stagger=False, middle_radius_delta=0.0):
    triangles = []
    rings = [ring(WIDTH, DEPTH, [r + middle_radius_delta if r else 0.0 for r in radii] if z == 5.0 else radii, z, stagger=stagger and z == 5.0)
             for z in (0.0, 5.0, HEIGHT)]
    cap(triangles, rings[0], downward=True)
    cap(triangles, rings[-1])
    for lower, upper in zip(rings, rings[1:]):
        wall(triangles, lower, upper)
    return triangles


def tray():
    triangles = []
    outer = [ring(WIDTH, DEPTH, [4.0] * 4, z) for z in (0.0, 6.0, HEIGHT)]
    inner = [ring(34.0, 24.0, [2.0] * 4, z) for z in (FLOOR, 7.5, HEIGHT)]
    cap(triangles, outer[0], downward=True)
    cap(triangles, inner[0])
    for lower, upper in zip(outer, outer[1:]):
        wall(triangles, lower, upper)
    for lower, upper in zip(inner, inner[1:]):
        wall(triangles, lower, upper, inner=True)
    # Actual annular source rim, including the hole: no covering disk.
    for i in range(len(outer[-1])):
        j = (i + 1) % len(outer[-1])
        a, b, c, d = outer[-1][i], outer[-1][j], inner[-1][j], inner[-1][i]
        triangles.extend([[a, b, c], [a, c, d]])
    return triangles


def transform(point, rotated):
    if not rotated:
        return point
    x, y, z = point
    sx, cx = math.sin(0.37), math.cos(0.37)
    sy, cy = math.sin(-0.51), math.cos(-0.51)
    y, z = cx * y - sx * z, sx * y + cx * z
    x, z = cy * x + sy * z, -sy * x + cy * z
    return (x + 4.0, y + 8.0, z + 12.0)


def signed_volume(triangles):
    def cross(a, b):
        return (a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0])
    return math.fsum(sum(a[i] * cross(b, c)[i] for i in range(3)) / 6
                     for a, b, c in triangles)


def nominal_bounds(radii, rotated):
    sx, cx = math.sin(0.37), math.cos(0.37)
    sy, cy = math.sin(-0.51), math.cos(-0.51)
    rows = ((cy, sy * sx, sy * cx), (0, cx, -sx), (-sy, cy * sx, cy * cx)) if rotated else ((1, 0, 0), (0, 1, 0), (0, 0, 1))
    shift = (4, 8, 12) if rotated else (0, 0, 0)
    bounds = [[], []]
    for axis, (a, b, c) in enumerate(rows):
        values = []
        for corner, (xs, ys) in enumerate(((1, -1), (1, 1), (-1, 1), (-1, -1))):
            radius = radii[corner]
            center = (xs * (WIDTH / 2 - radius), ys * (DEPTH / 2 - radius))
            first = (corner - 1) * math.pi / 2
            last = first + math.pi / 2
            angles = [first, last]
            for k in range(-3, 4):
                angle = math.atan2(b, a) + k * math.pi
                if first <= angle <= last:
                    angles.append(angle)
            values.extend(a * (center[0] + radius * math.cos(t)) + b * (center[1] + radius * math.sin(t)) for t in angles)
        bounds[0].append(shift[axis] + min(values) + min(0, c * HEIGHT))
        bounds[1].append(shift[axis] + max(values) + max(0, c * HEIGHT))
    return bounds


def write(name, triangles, reference, *, rotated=False):
    vertices = [transform(p, rotated) for t in triangles for p in t]
    lines = [f"solid {name}"]
    for index in range(0, len(vertices), 3):
        lines.extend(["facet normal 0 0 0", "outer loop"])
        lines.extend("vertex " + " ".join(format(x, ".17g") for x in p)
                     for p in vertices[index:index + 3])
        lines.extend(["endloop", "endfacet"])
    lines.append(f"endsolid {name}")
    data = ("\n".join(lines) + "\n").encode()
    (OUT / f"{name}.stl").write_bytes(data)
    if "profile_radii_mm" in reference:
        reference = {**reference, "bounds_mm": nominal_bounds(reference["profile_radii_mm"], rotated)}
    return {
        **reference,
        "rotated": rotated,
        "source_triangles": len(triangles),
        "source_polyhedral_volume_mm3": signed_volume(triangles),
        "sha256": hashlib.sha256(data).hexdigest(),
    }


def box_reference(radii):
    # A right angle loses a square minus its quarter disk, per rounded corner.
    rounded = [r for r in radii if r]
    return {
        "profile_radii_mm": radii,
        "planes": 6,
        "partial_cylinders": len(rounded),
        "inner_cylinders": 0,
        "radii_mm": sorted(rounded),
        "axial_lengths_mm": [HEIGHT] * len(rounded),
        "volume_mm3": (WIDTH * DEPTH - (1 - math.pi / 4) * sum(r * r for r in rounded)) * HEIGHT,
        "bounds_mm": [[-WIDTH / 2, -DEPTH / 2, 0.0], [WIDTH / 2, DEPTH / 2, HEIGHT]],
    }


one_radii = [4.0, 0.0, 0.0, 0.0]
two_radii = [4.0, 0.0, 2.0, 0.0]
one = box(one_radii)
two = box(two_radii)
open_tray = tray()
outer_area = WIDTH * DEPTH - (4 - math.pi) * 4.0**2
inner_area = 34.0 * 24.0 - (4 - math.pi) * 2.0**2
tray_reference = {
    "profile_radii_mm": [4.0] * 4,
    "planes": 11,
    "partial_cylinders": 8,
    "inner_cylinders": 4,
    "radii_mm": [2.0] * 4 + [4.0] * 4,
    "axial_lengths_mm": [HEIGHT - FLOOR] * 4 + [HEIGHT] * 4,
    "floor_thickness_mm": FLOOR,
    "outer_dimensions_mm": [WIDTH, DEPTH, HEIGHT],
    "inner_dimensions_mm": [34.0, 24.0, HEIGHT - FLOOR],
    "volume_mm3": outer_area * HEIGHT - inner_area * (HEIGHT - FLOOR),
    "bounds_mm": [[-WIDTH / 2, -DEPTH / 2, 0.0], [WIDTH / 2, DEPTH / 2, HEIGHT]],
    "probes": [
        {"point_mm": [0.371, 0.463, 7.6], "material": False, "role": "accessible cavity"},
        {"point_mm": [0.371, 0.463, 1.6], "material": True, "role": "source floor"},
        {"point_mm": [18.6, 0.463, 7.6], "material": True, "role": "straight wall"},
        {"point_mm": [17.5, 12.5, 7.6], "material": True, "role": "rounded corner wall"},
        {"point_mm": [21.371, 0.463, 7.6], "material": False, "role": "outside"},
    ],
}
cases = {
    "one_edge": write("one_edge", one, box_reference(one_radii)),
    "two_edges": write("two_edges", two, box_reference(two_radii)),
    "two_edges_staggered": write("two_edges_staggered", box(two_radii, stagger=True), box_reference(two_radii)),
    "two_edges_rotated": write("two_edges_rotated", two, box_reference(two_radii), rotated=True),
    "open_tray": write("open_tray", open_tray, tray_reference),
    "open_tray_rotated": write("open_tray_rotated", open_tray, tray_reference, rotated=True),
    # Two source end arcs still have R4, but the intermediate section is R4.25.
    # The boundary is closed and the two planar supports remain present. This
    # is deliberately not asserted to be a proper variable-blend envelope.
    "one_edge_variable_radius": write("one_edge_variable_radius", box(one_radii, middle_radius_delta=0.25), {
        "expected_refusal": "recognition.partial_unproven",
        "auto_partial_cylinders": 0,
        "end_radius_mm": 4.0,
        "middle_radius_mm": 4.25,
        "source_closed": True,
        "section_law": "piecewise linear R(z): (0,4), (5,4.25), (12,4), tangent planar supports retained",
    }),
}
# These are genuinely incomplete material boundaries, not open accessible voids.
for name, source, index in (("one_edge_missing_end", one, 0), ("tray_missing_floor", open_tray, 2 * len(profile(WIDTH, DEPTH, [4.0] * 4)) - 1)):
    defective = list(source)
    defective.pop(index)
    cases[name] = write(name, defective, {"expected_refusal": "mesh.open"})

(OUT / "manifest.json").write_text(json.dumps({
    "generator": "fixtures/generate_partial_fillets.py (Python standard library only)",
    "contract": "D1 disjoint straight-edge cylinders; E1 open tray with real floor/rim, vertical rounded corners only",
    "units": "mm",
    "segments_per_quarter": SEGMENTS,
    "rotation": {"x_rad": 0.37, "y_rad": -0.51, "translation_mm": [4.0, 8.0, 12.0]},
    "cases": cases,
}, indent=2) + "\n")
