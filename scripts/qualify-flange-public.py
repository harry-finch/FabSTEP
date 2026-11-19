"""Build an independent notched toroidal ramp and its source-only laboratory witness."""

import argparse
from pathlib import Path
import struct

import numpy as np


def refine_skin(points, triangles, rounds):
    """Refine the toric interior and keep the original shared boundary knots."""
    for _ in range(rounds):
        indices = triangles.ravel()
        pairs = np.sort(
            np.stack((indices, np.roll(triangles, -1, axis=1).ravel()), axis=1), axis=1
        )
        edges, inverse, count = np.unique(
            pairs, axis=0, return_inverse=True, return_counts=True
        )
        interior = count == 2
        ids = np.full(len(edges), -1)
        ids[interior] = len(points) + np.arange(np.count_nonzero(interior))
        sides = points[edges[interior]]
        u = np.arctan2(sides[:, :, 2], sides[:, :, 1]).mean(axis=1)
        v = np.arctan2(
            sides[:, :, 0], np.hypot(sides[:, :, 1], sides[:, :, 2]) - 20.0
        ).mean(axis=1)
        radius = 20.0 + 1.7 * np.cos(v)
        points = np.vstack(
            (
                points,
                np.stack(
                    (1.7 * np.sin(v), radius * np.cos(u), radius * np.sin(u)), axis=1
                ),
            )
        )
        refined = []
        for corners, mid in zip(triangles, ids[inverse].reshape(-1, 3)):
            if np.all(mid >= 0):
                refined.extend(
                    [[corners[k], mid[k], mid[(k - 1) % 3]] for k in range(3)]
                )
                refined.append(mid.tolist())
            elif np.all(mid < 0):
                refined.append(corners.tolist())
            elif np.count_nonzero(mid >= 0) == 1:
                k = int(np.nonzero(mid >= 0)[0][0])
                a, b, c = np.roll(corners, -k)
                refined.extend([[a, mid[k], c], [mid[k], b, c]])
            else:
                k = int(np.nonzero(mid < 0)[0][0])
                a, b, c = np.roll(corners, -k)
                m1, m2 = mid[(k + 1) % 3], mid[(k + 2) % 3]
                refined.extend([[a, b, m1], [a, m1, m2], [m2, m1, c]])
        triangles = np.array(refined)
    return points, triangles


def ramp(path, n, m, sparse_floor=False, subdivisions=0):
    # Height skin over an annular sector. Both radial notches interrupt the
    # toroidal chart. Vertical side walls and the planar bottom close it.
    R, r = 20.0, 1.7
    vertices = []
    bases = []
    for i in range(n + 1):
        u = 0.2 + 1.9 * i / n
        for j in range(m + 1):
            v = 0.12 + 1.23 * j / m
            rho = R + r * np.cos(v)
            vertices.append([r * np.sin(v), rho * np.cos(u), rho * np.sin(u)])
            bases.append([-0.8, rho * np.cos(u), rho * np.sin(u)])
    p = np.array(vertices + bases)
    N = len(vertices)
    top = []
    for i in range(n):
        for j in range(m):
            if (n // 3 <= i < n // 2 and j >= m // 2) or (
                2 * n // 3 <= i < 3 * n // 4 and j < m // 4
            ):
                continue
            a = i * (m + 1) + j
            b = a + m + 1
            c = b + 1
            d = a + 1
            top.extend([[a, b, c], [a, c, d]])
    T = np.array(top)
    a = T.ravel()
    b = np.roll(T, -1, axis=1).ravel()
    k = np.minimum(a, b) * N + np.maximum(a, b)
    _, i, counts = np.unique(k, return_index=True, return_counts=True)
    if sparse_floor:
        # Only the exterior contour of the bottom needs the top skin's knots.
        # Between two angular stations connect the ordered radial samples.
        # Notch endpoints retain their actual floor vertices on both strips.
        ends = np.union1d(a[i[counts == 1]], b[i[counts == 1]])
        stations = [sorted(ends[ends // (m + 1) == k] % (m + 1)) for k in range(n + 1)]
        floor = []
        for k in range(n):
            lo = m // 4 if 2 * n // 3 <= k < 3 * n // 4 else 0
            hi = m // 2 if n // 3 <= k < n // 2 else m
            rows = [[j for j in stations[k + side] if lo <= j <= hi] for side in (0, 1)]
            idx = [0, 0]

            def vertex(side):
                return N + (k + side) * (m + 1) + rows[side][idx[side]]

            while idx[0] + 1 < len(rows[0]) or idx[1] + 1 < len(rows[1]):
                left, right = vertex(0), vertex(1)
                advance_right = idx[0] + 1 == len(rows[0]) or (
                    idx[1] + 1 < len(rows[1])
                    and rows[1][idx[1] + 1] <= rows[0][idx[0] + 1]
                )
                side = int(advance_right)
                idx[side] += 1
                floor.append([vertex(side), right, left])
        if len(T) * 4**subdivisions + len(floor) + 2 * len(ends) > 600_000:
            raise ValueError("Reference exceeds the large-STL source limit")
        p, skin = refine_skin(p, T, subdivisions)
        out = skin.tolist() + floor
    else:
        out = T.tolist() + (T[:, ::-1] + N).tolist()
    for at in i[counts == 1]:
        x, y = a[at], b[at]
        out.extend([[x, y + N, y], [x, x + N, y + N]])
    points = p[np.array(out)]
    volume = (
        np.einsum("ij,ij->i", points[:, 0], np.cross(points[:, 1], points[:, 2])).sum()
        / 6
    )
    if volume < 0:
        points = points[:, ::-1]
    with path.open("wb") as f:
        f.write(bytes(80))
        f.write(struct.pack("<I", len(points)))
        for q in points:
            f.write(struct.pack("<12fH", 0, 0, 0, *q.ravel(), 0))


def witness(path, output):
    length = path.stat().st_size
    if not 84 <= length <= 84 + 100000 * 50:
        raise ValueError("Source-only laboratory size limit")
    with path.open("rb") as f:
        f.read(80)
        n = struct.unpack("<I", f.read(4))[0]
        if not n or length != 84 + n * 50:
            raise ValueError("Invalid binary STL length")
        dtype = np.dtype(
            [("normal", "<f4", (3,)), ("points", "<f4", (3, 3)), ("attribute", "<u2")]
        )
        points = np.fromfile(f, dtype=dtype, count=n)["points"].astype(float)
    if not np.isfinite(points).all():
        raise ValueError("Non-finite source vertex")
    p, inv = np.unique(points.reshape(-1, 3), axis=0, return_inverse=True)
    T = inv.reshape(-1, 3)
    a = T.ravel()
    b = np.roll(T, -1, axis=1).ravel()
    k = np.minimum(a, b) * len(p) + np.maximum(a, b)
    _, counts = np.unique(k, return_counts=True)
    if not (counts == 2).all():
        raise ValueError("Open or non-manifold measured STL")
    order = np.argsort(k).reshape(-1, 2)
    if not (a[order[:, 0]] == b[order[:, 1]]).all():
        raise ValueError("Inconsistent triangle orientation")
    q = points - p[0]
    volume = np.einsum("ij,ij->i", q[:, 0], np.cross(q[:, 1], q[:, 2])).sum() / 6
    area = (
        np.linalg.norm(np.cross(q[:, 1] - q[:, 0], q[:, 2] - q[:, 0]), axis=1).sum() / 2
    )
    if volume <= 0:
        raise ValueError("Expected one positively oriented source")
    with output.open("w") as f:
        print(len(p), len(T), 0, "%.17g" % volume, "%.17g" % area, file=f)
        np.savetxt(f, [p.min(0), p.max(0)], fmt="%.17g")
        np.savetxt(f, p, fmt="%.17g")
        np.savetxt(f, np.column_stack((np.arange(n), T)), fmt="%d")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    gen = sub.add_parser("ramp")
    gen.add_argument("output", type=Path)
    gen.add_argument("--u", type=int, default=60)
    gen.add_argument("--v", type=int, default=36)
    src = sub.add_parser("witness")
    gen.add_argument(
        "--sparse-floor",
        action="store_true",
        help="retain only contour knots on the plane, for dense toroidal test meshes",
    )
    gen.add_argument(
        "--subdivisions",
        type=int,
        choices=range(4),
        default=0,
        help="refine the toric skin of a sparse-floor reference",
    )
    src.add_argument("stl", type=Path)
    src.add_argument("output", type=Path)
    args = parser.parse_args()
    if args.action == "ramp":
        if not 24 <= args.u <= 150 or not 16 <= args.v <= 100:
            raise ValueError("Unsupported angular resolution")
        if args.subdivisions and not args.sparse_floor:
            parser.error("--subdivisions requires --sparse-floor")
        ramp(args.output, args.u, args.v, args.sparse_floor, args.subdivisions)
    else:
        witness(args.stl, args.output)
