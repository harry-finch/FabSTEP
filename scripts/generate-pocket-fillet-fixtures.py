#!/usr/bin/env python3
"""Extrude a measured pocket section into a block with one internal floor fillet."""

import argparse
from collections import Counter, defaultdict
from dataclasses import dataclass
import hashlib
import json
import math
from pathlib import Path
import struct


OUT = Path(__file__).resolve().parents[1] / "fixtures" / "pocket_fillet"
FACET = struct.Struct("<12fH")
GROUPS = (
    "outer_bottom",
    "outer_y_min",
    "outer_x_max",
    "outer_y_max",
    "outer_x_min",
    "rim",
    "floor",
    "fillet",
    "tangent_wall",
    "opposite_wall",
    "stop_min",
    "stop_max",
)


@dataclass(frozen=True)
class Dimensions:
    length: float = 30
    width: float = 20
    height: float = 12
    pocket_length: float = 16
    pocket_width: float = 10
    floor: float = 3
    radius: float = 2


def rectangle(length, width, z, divisions):
    """CCW perimeter with the same stations as the elongated walls."""
    y = width / 2
    xs = [-length / 2 + length * i / divisions for i in range(divisions + 1)]
    return [(x, -y, z) for x in xs] + [(x, y, z) for x in reversed(xs)]


def place(point, name):
    x, y, z = point
    if name in ("rotated", "rotated_f32"):
        # Two rigid rotations (about Z then Y) and a translation.
        u, v = (4 * x - 3 * y) / 5, (3 * x + 4 * y) / 5
        return ((12 * u + 5 * z) / 13 + 4, v - 3, (12 * z - 5 * u) / 13 + 2)
    if name == "opposite":
        return (-x, -y, z)
    return point


def generate(name, d, nx, na):
    groups = defaultdict(list)

    def quad(group, a, b, c, e, alternate=False):
        faces = [(a, b, e), (b, c, e)] if alternate else [(a, b, c), (a, c, e)]
        groups[group].extend(faces)

    base = rectangle(d.length, d.width, 0, nx)
    top = rectangle(d.length, d.width, d.height, nx)
    mouth = rectangle(d.pocket_length, d.pocket_width, d.height, nx)
    for i in range(len(top)):
        j = (i + 1) % len(top)
        groups["outer_bottom"].append(((0, 0, 0), base[j], base[i]))
        group = (
            "outer_y_min"
            if i < nx
            else "outer_x_max"
            if i == nx
            else "outer_y_max"
            if i < 2 * nx + 1
            else "outer_x_min"
        )
        quad(group, base[i], base[j], top[j], top[i])
        quad("rim", top[i], top[j], mouth[j], mouth[i])

    ymin = -d.pocket_width / 2
    cy, cz = ymin + d.radius, d.floor + d.radius
    # The boundary of the removed section follows the tangent wall, then
    # the concave quarter circle, the level floor and the opposing sharp wall.
    rows = []
    for i in range(nx + 1):
        steps = na + (7 * i if name in ("staggered", "warped_stop") else 0)
        section = [(ymin, d.height), (ymin, cz)]
        section.extend(
            (
                cy - d.radius * math.cos(j * math.pi / (2 * steps)),
                cz - d.radius * math.sin(j * math.pi / (2 * steps)),
            )
            for j in range(1, steps)
        )
        section.extend([(cy, d.floor), (-ymin, d.floor), (-ymin, d.height)])
        x = -d.pocket_length / 2 + d.pocket_length * i / nx
        ring = [(x, y, z) for y, z in section]
        if (name == "warped" and i == nx // 2) or (name == "warped_stop" and i == nx):
            j = 1 + steps // 2
            _, y, z = ring[j]
            scale = 1 + 0.16 / d.radius
            ring[j] = (x, cy + (y - cy) * scale, cz + (z - cz) * scale)
        rows.append(ring)
    for i, (left, right) in enumerate(zip(rows, rows[1:])):
        for j, group in [(0, "tangent_wall"), (-3, "floor"), (-2, "opposite_wall")]:
            if name != "missing_floor" or group != "floor":
                quad(group, left[j], right[j], right[j + 1], left[j + 1])
        # Walk both arcs in increasing angle. Their angular knots need not
        # coincide. Where they do, either diagonal represents the same strip.
        a, b = left[1:-2], right[1:-2]
        n, m = len(a) - 1, len(b) - 1
        j = k = 0
        while j < n or k < m:
            advance_left = (j + 1) * m
            advance_right = (k + 1) * n
            if advance_left == advance_right:
                quad(
                    "fillet",
                    a[j],
                    b[k],
                    b[k + 1],
                    a[j + 1],
                    alternate=name == "retessellated" and (i + j) % 2 == 0,
                )
                j += 1
                k += 1
            elif advance_left < advance_right:
                groups["fillet"].append((a[j], b[k], a[j + 1]))
                j += 1
            else:
                groups["fillet"].append((a[j], b[k], b[k + 1]))
                k += 1
    for idx, group in ((0, "stop_min"), (-1, "stop_max")):
        ring = rows[idx]
        # The section is convex. Its fan stays on the actual end wall and
        # includes the top edge at the mouth.
        center = (ring[0][0], 0, (d.height + d.floor) / 2)
        for a, b in zip(ring, ring[1:] + ring[:1]):
            groups[group].append((center, a, b) if idx == 0 else (center, b, a))

    facets = []
    ranges = {}
    for group in GROUPS:
        start = len(facets)
        facets.extend(tuple(place(p, name) for p in t) for t in groups[group])
        ranges[group] = [start, len(facets)]
    return facets, ranges


def signed_tetrahedron(a, b, c):
    return (
        a[0] * (b[1] * c[2] - b[2] * c[1])
        + a[1] * (b[2] * c[0] - b[0] * c[2])
        + a[2] * (b[0] * c[1] - b[1] * c[0])
    ) / 6


def write(directory, name, d, nx, na):
    faces, groups = generate(name, d, nx, na)
    if name == "rotated":
        lines = ["solid pocket"]
        for tri in faces:
            lines.extend(["facet normal 0 0 0", "outer loop"])
            lines.extend(
                "vertex " + " ".join(format(x, ".17g") for x in p) for p in tri
            )
            lines.extend(["endloop", "endfacet"])
        lines.append("endsolid pocket")
        data = ("\n".join(lines) + "\n").encode()
        vertices = (
            tuple(map(float, line.split()[1:]))
            for line in data.splitlines()
            if line.startswith(b"vertex ")
        )
        triangles = zip(vertices, vertices, vertices)
    else:
        data = bytearray(80) + struct.pack("<I", len(faces))
        for tri in faces:
            data.extend(FACET.pack(0, 0, 0, *(x for p in tri for x in p), 0))
        triangles = ((t[3:6], t[6:9], t[9:12]) for t in FACET.iter_unpack(data[84:]))
    edges = Counter()
    volumes = []
    for a, b, c in triangles:
        assert len({a, b, c}) == 3
        edges.update([(a, b), (b, c), (c, a)])
        volumes.append(signed_tetrahedron(a, b, c))
    boundary = sum((b, a) not in edges for a, b in edges)
    assert max(edges.values()) == 1
    assert boundary == (2 * nx + 2 if name == "missing_floor" else 0)
    path = directory / (name + ".stl")
    path.write_bytes(data)
    case = dict(
        triangles=len(faces),
        groups=groups,
        open_edges=boundary,
        sha256=hashlib.sha256(data).hexdigest(),
    )
    if name in ("missing_floor", "warped", "warped_stop"):
        return case

    cy, cz = -d.pocket_width / 2 + d.radius, d.floor + d.radius
    # Positive correction is the sliver of material retained by the fillet.
    volume = (
        d.length * d.width * d.height
        - d.pocket_length * d.pocket_width * (d.height - d.floor)
        + d.pocket_length * d.radius**2 * (1 - math.pi / 4)
    )
    chord_error = (
        d.pocket_length
        * d.radius**2
        * (math.pi / 2 - na * math.sin(math.pi / (2 * na)))
        / 2
    )
    excess = math.fsum(volumes) - volume
    if name == "staggered":
        # Every triangle lies radially outside the inscribed circle defined
        # by the coarser angular step. Bound the missing sector of void.
        shell_volume = (
            d.pocket_length
            * math.pi
            * d.radius**2
            / 4
            * math.sin(math.pi / (4 * na)) ** 2
        )
        assert 0 < excess < shell_volume + 0.002
    else:
        assert abs(excess - chord_error) < 0.002
    corners = [
        place((x, y, z), name)
        for x in (-d.length / 2, d.length / 2)
        for y in (-d.width / 2, d.width / 2)
        for z in (0, d.height)
    ]
    case.update(
        dimensions_mm=vars(d),
        axis_centers_mm=[
            place((x, cy, cz), name)
            for x in (-d.pocket_length / 2, d.pocket_length / 2)
        ],
        radius_mm=d.radius,
        bounds_mm=[[f(c[i] for c in corners) for i in range(3)] for f in (min, max)],
        volume_mm3=volume,
        area_fillet_mm2=math.pi * d.radius * d.pocket_length / 2,
        inside_void_mm=place((0, 0, (d.floor + d.height) / 2), name),
        inside_material_mm=place((0, cy - 0.85 * d.radius, cz - 0.85 * d.radius), name),
    )
    return case


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, nargs="?", default=OUT)
    out = parser.parse_args().output
    out.mkdir(parents=True, exist_ok=True)
    nominal = Dimensions()
    opposite = Dimensions(38, 24, 14, 20, 14, 4, 3)
    cases = {}
    for name, dims, nx, na in [
        ("nominal", nominal, 2, 16),
        ("rotated", nominal, 2, 16),
        ("rotated_f32", nominal, 2, 16),
        ("retessellated", nominal, 3, 23),
        ("opposite", opposite, 4, 20),
        ("staggered", nominal, 1, 16),
        ("warped_stop", nominal, 1, 16),
        ("missing_floor", nominal, 2, 16),
        ("warped", nominal, 2, 16),
    ]:
        cases[name] = write(out, name, dims, nx, na)
    manifest = dict(
        unit="mm",
        positives=["nominal", "rotated", "retessellated", "opposite", "staggered"],
        negatives=["missing_floor", "warped", "warped_stop"],
        precision_controls=["rotated_f32"],
        planes=11,
        cylinders=1,
        cases=cases,
    )
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
