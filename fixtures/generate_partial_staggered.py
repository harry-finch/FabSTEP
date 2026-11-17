#!/usr/bin/env python3
"""Independent open-cylinder tessellations; no FabSTEP recognition dependency."""
import json
import math
import struct
from pathlib import Path

OUT = Path(__file__).parent / 'partial_staggered'
OUT.mkdir(exist_ok=True)


def write(name, rings, cap_center, *, quantize=False, translation=(0, 0), missing=False, rotate=False):
    triangles = []
    for i in range(len(rings[0])):
        j = (i + 1) % len(rings[0])
        triangles.append([(cap_center[0], cap_center[1], 0), rings[0][j], rings[0][i]])
        triangles.append([(cap_center[0], cap_center[1], 7), rings[-1][i], rings[-1][j]])
        for lower, upper in zip(rings, rings[1:]):
            triangles.extend([[lower[i], lower[j], upper[j]], [lower[i], upper[j], upper[i]]])
    if missing:
        triangles.pop(20)
    def coordinate(p):
        if rotate:
            ax, ay = .37, -.51
            x, y, z = p
            y, z = math.cos(ax) * y - math.sin(ax) * z, math.sin(ax) * y + math.cos(ax) * z
            x, z = math.cos(ay) * x + math.sin(ay) * z, -math.sin(ay) * x + math.cos(ay) * z
            p = (x + 4, y + 8, z + 12)
        p = (p[0] + translation[0], p[1] + translation[1], p[2])
        return tuple(struct.unpack('<f', struct.pack('<f', v))[0] for v in p) if quantize else p
    lines = ['solid independent_staggered']
    for tri in triangles:
        lines.extend(['facet normal 0 0 0', 'outer loop'])
        lines.extend('vertex ' + ' '.join(format(v, '.17g') for v in coordinate(p)) for p in tri)
        lines.extend(['endloop', 'endfacet'])
    lines.append('endsolid independent_staggered')
    (OUT / (name + '.stl')).write_text('\n'.join(lines) + '\n')


def d_rings(defect=None, delayed=False):
    n = 24
    rings = []
    for layer, z in enumerate((0, 1, 2.8, 6, 7) if delayed else (0, 2.8, 7)):
        middle = z == 2.8
        ring = []
        for i in range(n + 1):
            angle = math.pi * i / n
            if middle:
                angle += .32 * math.pi / n * math.sin(math.pi * i / n)
            radius = 5 + ((-.02 if defect == 'dent' else .02) if defect in ('bulge', 'dent') and layer == 1 and i == 12 else 0)
            if defect == 'excursion' and layer == 1 and i == 1:
                angle = -.03
            ring.append((radius * math.cos(angle), radius * math.sin(angle), z))
        if defect == 'fold' and layer == 1:
            ring[10], ring[12] = ring[12], ring[10]
        rings.append(ring)
    return rings


write('d_staggered', d_rings(), (0, 1))
write('d_staggered_delayed', d_rings(delayed=True), (0, 1))
write('d_staggered_rotated', d_rings(), (0, 1), rotate=True)
write('d_staggered_f32_translated', d_rings(), (0, 1), quantize=True, translation=(73, -133))
write('d_bulge', d_rings('bulge'), (0, 1))
write('d_dent', d_rings('dent'), (0, 1))
write('d_excursion', d_rings('excursion'), (0, 1))
write('d_fold', d_rings('fold'), (0, 1))
write('d_missing', d_rings(), (0, 1), missing=True)

# Convex lens bounded by two distinct circular arcs and two shared straight
# generators; retain the immediate rail neighborhood at the same angles.
n = 20
alpha = math.acos(2 / 3)
y = math.sqrt(5)
rings = []
for layer, z in enumerate((0, 2.8, 7)):
    ring = []
    for side in range(2):
        for i in range(n):
            angle = (-alpha if side == 0 else math.pi - alpha) + 2 * alpha * i / n
            if layer == 1 and 1 < i < n - 1:
                angle += .2 * 2 * alpha / n * math.sin(math.pi * i / n)
            x = (-2 if side == 0 else 2) + 3 * math.cos(angle)
            yy = 3 * math.sin(angle)
            if i == 0:
                x, yy = 0, -y if side == 0 else y
            ring.append((x, yy, z))
    rings.append(ring)
write('lens_shared_rails', rings, (0, 0))

(OUT / 'expected.json').write_text(json.dumps({
    'height_mm': 7,
    'd_volume_mm3': 7 * math.pi * 25 / 2,
    'lens_volume_mm3': 7 * (18 * alpha - 4 * math.sqrt(5)),
    'rotation_xyz': {'x_rad': .37, 'y_rad': -.51, 'translation_mm': [4, 8, 12]},
    'positive': ['d_staggered', 'd_staggered_delayed', 'd_staggered_rotated', 'd_staggered_f32_translated', 'lens_shared_rails'],
    'negative': ['d_bulge', 'd_dent', 'd_excursion', 'd_fold', 'd_missing'],
}, indent=2) + '\n')
