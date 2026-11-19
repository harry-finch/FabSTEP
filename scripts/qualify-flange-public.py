"""Build an independent notched toroidal ramp and its source-only laboratory witness."""

import argparse
from pathlib import Path
import struct

import numpy as np


def ramp(path, n, m):
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
    src.add_argument("stl", type=Path)
    src.add_argument("output", type=Path)
    args = parser.parse_args()
    if args.action == "ramp":
        if not 24 <= args.u <= 150 or not 16 <= args.v <= 100:
            raise ValueError("Unsupported angular resolution")
        ramp(args.output, args.u, args.v)
    else:
        witness(args.stl, args.output)
