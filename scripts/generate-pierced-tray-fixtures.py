#!/usr/bin/env python3
"""E2 independent analytic tray and bore; stdlib only, no CAD dependency."""
import hashlib
import json
import math
from pathlib import Path

OUT = Path(__file__).resolve().parents[1] / 'fixtures' / 'pierced_tray'


def outline(width, depth, radius, z):
    points = []
    for corner, (sx, sy) in enumerate(((1, -1), (1, 1), (-1, 1), (-1, -1))):
        for i in range(17):
            a = (corner - 1 + i / 16) * math.pi / 2
            points.append((sx * (width / 2 - radius) + radius * math.cos(a),
                           sy * (depth / 2 - radius) + radius * math.sin(a), z))
    # Subdivide long straight sides as well; then radial annular connections
    # also give the independent bore a resolved angular tessellation.
    resolved = []
    for i, p in enumerate(points):
        q = points[(i + 1) % len(points)]
        resolved.append(p)
        if (i + 1) % 17 == 0:
            resolved.extend(tuple(p[k] + (q[k] - p[k]) * j / 32 for k in range(3)) for j in range(1, 32))
    return resolved


def join(tris, lower, upper, reverse=False):
    assert len(lower) == len(upper)
    for i in range(len(lower)):
        j = (i + 1) % len(lower)
        pair = [(lower[i], lower[j], upper[j]), (lower[i], upper[j], upper[i])]
        tris.extend(tuple(reversed(t)) if reverse else t for t in pair)


def build(warped=False):
    outer = [outline(40, 30, 4, z) for z in (0, 6, 12)]
    inner = [outline(34, 24, 2, z) for z in (3, 7.5, 12)]
    # Match source rays on the nested convex boundaries to avoid crossing
    # annular triangulations; the subdivided sides bound the bore chord error.
    angles = [math.atan2(y, x) for x, y, _ in outer[0]]
    holes = [[((2 + (0.05 if warped and z == 1.5 else 0)) * math.cos(a),
               (2 + (0.05 if warped and z == 1.5 else 0)) * math.sin(a), z)
              for a in angles] for z in (0, 1.5, 3)]
    tris = []
    for rings, reverse in ((outer, False), (inner, True), (holes, True)):
        for lo, hi in zip(rings, rings[1:]):
            join(tris, lo, hi, reverse)
    join(tris, outer[-1], inner[-1])  # rim
    join(tris, outer[0], holes[0], True)  # bottom annulus
    join(tris, inner[0], holes[-1])  # cavity floor annulus
    return tris


def transform(p):
    x, y, z = p
    return (y + 10, z - 4, x + 5)  # exact rigid permutation and translation


def write(name, tris, rotate=False):
    lines = [f'solid {name}']
    for t in tris:
        lines += ['facet normal 0 0 0', 'outer loop']
        lines += ['vertex ' + ' '.join(format(x, '.17g') for x in (transform(p) if rotate else p)) for p in t]
        lines += ['endloop', 'endfacet']
    lines += [f'endsolid {name}']
    data = ('\n'.join(lines) + '\n').encode()
    (OUT / (name + '.stl')).write_bytes(data)
    return {'triangles': len(tris), 'sha256': hashlib.sha256(data).hexdigest()}


def main():
    OUT.mkdir(exist_ok=True)
    tris = build()
    cases = {name: write(name, mesh, rotate) for name, mesh, rotate in (
        ('nominal', tris, False), ('rotated', tris, True),
        ('warped_bore', build(True), False), ('missing_floor', tris[:-1], False))}
    outer_area = 40 * 30 - (4 - math.pi) * 4**2
    inner_area = 34 * 24 - (4 - math.pi) * 2**2
    (OUT / 'manifest.json').write_text(json.dumps({
        'generator': 'scripts/generate-pierced-tray-fixtures.py', 'unit': 'mm',
        'volume_mm3': outer_area * 12 - inner_area * 9 - math.pi * 2**2 * 3,
        'bounds_mm': [[-20,-15,0],[20,15,12]], 'planes': 11, 'cylinders': 9,
        'bore_radius_mm': 2, 'floor_thickness_mm': 3, 'cases': cases,
    }, indent=2) + '\n')


if __name__ == '__main__':
    main()
