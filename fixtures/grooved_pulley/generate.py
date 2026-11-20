"""Circumferential groove on a pulley. The dimensions are independent of FabSTEP."""
import math
from pathlib import Path
import struct


def write(path, sides, stations, flaw=None, radius=9.0, minor=1.5, sweep=math.pi):
    # The circular arc starts at its lower shoulder, passes inside the pulley,
    # and rejoins its two outer cylindrical lands at exactly the same radius.
    center = radius + minor * math.cos(sweep * 0.5)
    samples = [(0.0, radius, 0.0)]
    for j in range(stations + 1):
        angle = math.pi + sweep * (0.5 - j / stations)
        r = center + minor * math.cos(angle)
        shift = 0.0
        if j == stations // 2:
            if flaw == "eccentric":
                shift = 0.08
            if flaw == "off_circle":
                r += 0.08
        samples.append((5.0 + minor * math.sin(angle), r, shift))
    samples.append((9.0, radius, 0.0))
    rings = [
        [(r * math.cos(math.tau * i / sides) + dx,
          r * math.sin(math.tau * i / sides), z) for i in range(sides)]
        for z, r, dx in samples
    ]
    tris = []
    for i in range(sides):
        k = (i + 1) % sides
        tris.append(((0, 0, 0), rings[0][k], rings[0][i]))
        tris.append(((0, 0, 9), rings[-1][i], rings[-1][k]))
        for a, b in zip(rings, rings[1:]):
            tris.extend(((a[i], a[k], b[k]), (a[i], b[k], b[i])))
    with path.open('wb') as out:
        out.write(bytes(80) + struct.pack('<I', len(tris)))
        for tri in tris:
            out.write(struct.pack('<12fH', 0, 0, 0, *tri[0], *tri[1], *tri[2], 0))


if __name__ == "__main__":
    root = Path(__file__).parent
    for name, sides, stations in [('coarse', 56, 12), ('fine', 80, 20)]:
        write(root / f'semicircle-{name}.stl', sides, stations)
        write(root / f'shallow-{name}.stl', sides, stations, sweep=2.0)
    for flaw in ['eccentric', 'off_circle']:
        write(root / f'{flaw}.stl', 56, 12, flaw=flaw)
