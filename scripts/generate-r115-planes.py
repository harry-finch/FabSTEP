"""Generate the two rotated f32 solids used by quantized_planes.rs, in millimetres."""

from math import cos, sin, sqrt
from pathlib import Path
import struct


ROOT = Path(__file__).resolve().parents[1] / "fixtures"
XS = (0, 0.08, 2, 7, 14)
YS = (0, 0.08, 2, 6, 10)
AXIS = tuple(k / sqrt(14) for k in (1, 3, 2))


def rotate(point):
    x, y, z = point
    a, b, c = AXIS
    dot = a * x + b * y + c * z
    cross = (b * z - c * y, c * x - a * z, a * y - b * x)
    return tuple(
        (shift + v * cos(0.3) + w * sin(0.3) + axis * dot * (1 - cos(0.3))) / 16
        for v, w, axis, shift in zip(point, cross, AXIS, (1.4, -0.6, 0.2))
    )


def write_solid(name, slope):
    triangles = []

    def vertex(i, j, top):
        return rotate((XS[i], YS[j], 2 + slope * max(0, XS[i] - 7) if top else 0))

    def quad(a, b, c, d, reverse=False):
        if reverse:
            b, d = d, b
        triangles.extend(((a, b, c), (a, c, d)))

    # Start with small f32 facets. Keep each source ID tied to its nominal face.
    for i in range(4):
        for j in range(4):
            for top in (True, False):
                quad(vertex(i, j, top), vertex(i + 1, j, top),
                     vertex(i + 1, j + 1, top), vertex(i, j + 1, top), not top)
    for i in range(4):
        for j in (0, 4):
            quad(vertex(i, j, False), vertex(i + 1, j, False),
                 vertex(i + 1, j, True), vertex(i, j, True), j == 4)
    for j in range(4):
        for i in (0, 4):
            quad(vertex(i, j, False), vertex(i, j + 1, False),
                 vertex(i, j + 1, True), vertex(i, j, True), i == 0)

    with (ROOT / f"{name}.stl").open("wb") as stl:
        stl.write(name.encode().ljust(80, b"\0") + struct.pack("<I", len(triangles)))
        for a, b, c in triangles:
            stl.write(struct.pack("<12fH", 0, 0, 0, *a, *b, *c, 0))


if __name__ == "__main__":
    write_solid("r115-rotated-block", 0)
    write_solid("r115-rotated-crease", 0.125)
