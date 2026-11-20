"""Generate a closed annular sector with two toroidal skins and a narrow staggered seam."""

import argparse
import math
from pathlib import Path
import struct


def torus_point(minor, rim, height, u, v):
    rho = rim + minor * math.cos(v)
    return (height + minor * (math.sin(v) - 1), rho * math.cos(u), rho * math.sin(u))


def generate(output, dense=False, mixed_density=False, stations=32):
    # Independent meridian radii. At their upper rims the shoulders are only
    # 0.008 mm apart radially and differ by 0.00006 mm axially. Staggering
    # angular stations makes the connecting triangle fans nearly tangent.
    steps = stations
    meridians = 48
    vertices = []
    angles = []
    patches = []
    for minor, rim, height, start_v, end_v, phase in [
        (2.4, 32.0, 0.0, 2.76, math.pi / 2, 0.0),
        (1.8, 32.008, -0.00006, math.pi / 2, 0.34, -0.00092),
    ]:
        patches.append((minor, rim, height))
        for j in range(meridians + 1):
            v = start_v + (end_v - start_v) * j / meridians
            for i in range(steps + 1):
                u = 0.35 + 0.021 * i + phase
                angles.append((u, v))
                vertices.append(torus_point(minor, rim, height, u, v))

    # Use the same orientation for the skins and the intervening narrow strip.
    top = []
    ends = {}
    for j in range(2 * (meridians + 1) - 1):
        for i in range(steps):
            a = j * (steps + 1) + i
            b = a + steps + 1
            for face in [(a, b, a + 1), (a + 1, b, b + 1)]:
                top.append(face)
                for v, w in zip(face, face[1:] + face[:1]):
                    if (w, v) in ends:
                        del ends[w, v]
                    else:
                        ends[v, w] = True

    n = len(vertices)
    vertices += [(-4.0, y - 0.15, z + 0.09) for _, y, z in vertices]
    # Only the contour stations are needed on the planar bottom. Keep the
    # two seam rows throughout, as their angular stations are staggered.
    last = 2 * meridians + 1
    middle = [0, meridians, meridians + 1, last]
    floor = []
    for i in range(steps):
        rows = [range(last + 1) if k in (0, steps) else middle for k in (i, i + 1)]
        left = right = 0
        while left + 1 < len(rows[0]) or right + 1 < len(rows[1]):
            a = n + rows[0][left] * (steps + 1) + i
            b = n + rows[1][right] * (steps + 1) + i + 1
            if right + 1 == len(rows[1]) or (
                left + 1 < len(rows[0]) and rows[0][left + 1] <= rows[1][right + 1]
            ):
                left += 1
                c = n + rows[0][left] * (steps + 1) + i
            else:
                right += 1
                c = n + rows[1][right] * (steps + 1) + i + 1
            floor.append((a, b, c))
    triangles = top + floor
    for a, b in ends:
        triangles.extend([(b, a, a + n), (b, a + n, b + n)])

    if dense:
        # Insert barycentres in the material coordinates of each torus.
        # Keeping all edge stations preserves the exact same seam and base.
        triangles = triangles[len(top):]
        angles += [None] * n
        for index, face in enumerate(top):
            row = min(face) // (steps + 1)
            if row == meridians:
                triangles.append(face)
                continue
            patch = patches[int(row > meridians)]
            refined = [face]
            # Barycentres stay strictly interior. Alternating the final level
            # changes only density and leaves the shared source edges intact.
            levels = 2 if mixed_density and index % 2 else 3
            for _ in range(levels):
                children = []
                for a, b, c in refined:
                    u, v = (sum(angles[i][k] for i in (a, b, c)) / 3 for k in (0, 1))
                    center = len(vertices)
                    vertices.append(torus_point(*patch, u, v))
                    angles.append((u, v))
                    children.extend([(a, b, center), (b, c, center), (c, a, center)])
                refined = children
            triangles.extend(refined)

    with output.open("wb") as stream:
        stream.write(bytes(80))
        stream.write(struct.pack("<I", len(triangles)))
        for face in triangles:
            xyz = [value for vertex in face for value in vertices[vertex]]
            stream.write(struct.pack("<12fH", 0, 0, 0, *xyz, 0))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--dense", action="store_true", help="exercise the large-STL worker on the same collar")
    parser.add_argument("--mixed-density", action="store_true", help="alternate two and three interior refinement levels with --dense")
    parser.add_argument("--stations", type=int, choices=(27,32), default=32, help="public R-131 collar benchmark")
    args = parser.parse_args()
    if args.mixed_density and not args.dense:
        parser.error("--mixed-density requires --dense")
    generate(args.output, args.dense, args.mixed_density, args.stations)
