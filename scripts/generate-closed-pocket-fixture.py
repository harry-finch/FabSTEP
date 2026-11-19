"""Generate an independent watertight mesh of a pocket from rectangular height sections."""

import math
from pathlib import Path
import struct
import sys


def rectangle(x, y, w, length, z):
    return [(x, y, z), (x + w, y, z), (x + w, y + length, z), (x, y + length, z)]


def write(path):
    facets = []

    def quad(a, b, c, d, flip=False):
        facets.extend([(a, b, d), (b, c, d)] if flip else [(a, b, c), (a, c, d)])

    base = rectangle(0, 0, 44, 36, 0)
    lid = rectangle(0, 0, 44, 36, 14)
    opening = rectangle(5, 6, 22, 18, 14)
    layers = []
    n = 27
    for k in range(n + 1):
        t = (k / n) ** 1.13 * math.pi / 2
        d = 2.5 * (1 - math.sin(t))
        z = 3 + 2.5 * (1 - math.cos(t))
        layers.append(rectangle(5 + d, 6 + d, 22 - 2 * d, 18 - 2 * d, z))
    quad(*reversed(base))
    quad(*layers[0])
    for i in range(4):
        j = (i + 1) % 4
        quad(base[i], base[j], lid[j], lid[i])
        quad(lid[i], lid[j], opening[j], opening[i])
        quad(layers[-1][i], opening[i], opening[j], layers[-1][j])
        for k, (lower, upper) in enumerate(zip(layers, layers[1:])):
            quad(lower[i], upper[i], upper[j], lower[j], (k + i) % 2 == 1)
    with path.open("wb") as out:
        out.write(
            b"FabSTEP closed pocket, independent rectangular height sections".ljust(
                80, b" "
            )
        )
        out.write(struct.pack("<I", len(facets)))
        for pts in facets:
            out.write(
                struct.pack("<12fH", 0.0, 0.0, 0.0, *(v for p in pts for v in p), 0)
            )


if __name__ == "__main__":
    write(Path(sys.argv[1]))
