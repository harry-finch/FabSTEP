"""Fit torus hypotheses to measured ring sections in a residual mesh."""

from pathlib import Path
import argparse
import json
import runpy

import numpy as np
from scipy.optimize import least_squares
from scipy.sparse import coo_matrix
from scipy.sparse.csgraph import connected_components
from scipy.spatial import cKDTree

util = runpy.run_path(str(Path(__file__).with_name("qualify-flange-residual-tori.py")))
require = util["require"]


def circumcenter(q):
    mean = q.mean(axis=0)
    pts = q - mean
    offsets, _, rank, _ = np.linalg.lstsq(2 * pts, (pts * pts).sum(axis=1), rcond=None)
    if rank != 2:
        return None
    radii = np.linalg.norm(pts - offsets, axis=1)
    return mean + offsets, radii.mean(), np.ptp(radii)


def ring_centers(p, tri):
    # Laboratory frame X is the lathe axis. Only connected source edges can
    # define a cross-section. Independent near-coincident rings stay separate.
    a = tri.ravel()
    b = np.roll(tri, -1, axis=1).ravel()
    use = (np.abs(p[a, 0] - p[b, 0]) < 5e-6) & (a < b)
    _, labels = connected_components(
        coo_matrix((np.ones(use.sum()), (a[use], b[use])), shape=(len(p), len(p))),
        directed=False,
    )
    circles = []
    witnesses = []
    for ids in np.split(np.argsort(labels), np.cumsum(np.bincount(labels))[:-1]):
        if len(ids) < 8 or np.ptp(p[ids, 0]) > 3e-5:
            continue
        fit = circumcenter(p[ids, 1:])
        if fit is None:
            continue
        center, r, dr = fit
        if r > 0.05 and dr < max(r * 3e-6, 3e-5):
            circles.append(center)
            witnesses.append(ids)
    if len(circles) < 8:
        return []
    pts = np.array(circles)
    require(len(pts) <= 25_000, "Ring candidate budget")
    # Bucket counts first to avoid constructing a dense all-pairs graph for
    # repeated estimates of the same axis (often hundreds per ring).
    bins = np.floor(pts / 0.005).astype(int)
    keys, inv, counts = np.unique(bins, axis=0, return_counts=True, return_inverse=True)
    near = cKDTree(keys).query_pairs(1.5, output_type="ndarray")
    _, groups = connected_components(
        coo_matrix((np.ones(len(near)), near.T), shape=(len(keys), len(keys))),
        directed=False,
    )
    labels = groups[inv]
    sizes = np.bincount(labels)
    roots = np.argsort(sizes)[::-1][:16]
    return [
        (
            np.median(pts[labels == i], axis=0),
            np.concatenate([witnesses[j] for j in np.flatnonzero(labels == i)]),
        )
        for i in roots
        if sizes[i] >= 8
    ]


def refine(points, h):
    def residual(model, pts):
        x, y, z, R, r = model
        rho = np.hypot(pts[:, 1] - y, pts[:, 2] - z)
        return np.hypot(pts[:, 0] - x, rho - R) - r

    for band in (0.02, 0.002, 0.0002, 2e-5):
        ok = np.abs(residual(h, points)) < band
        src = points[ok]
        if len(src) < 80:
            return None
        # Bound nonlinear fitting cost independently of source mesh size.
        stride = max(1, len(src) // 1500)
        sample = src[::stride]
        h = least_squares(
            residual, h, args=(sample,), loss="soft_l1", f_scale=band / 10, max_nfev=30
        ).x
        if not (h[3] > h[4] > 0.05) or not np.isfinite(h).all():
            return None
    err = np.abs(residual(h, points))
    return h, err


def find(p, tri):
    rng = np.random.default_rng(117)
    hypotheses = []
    for center, ring_ids in ring_centers(p, tri):
        cy, cz = center
        rho = np.hypot(p[:, 1] - cy, p[:, 2] - cz)
        q = np.column_stack((p[:, 0], rho))
        available = np.ones(len(p), dtype=bool)
        for _ in range(4):
            choose = np.flatnonzero(available)
            if len(choose) < 100:
                break
            seeds = ring_ids[available[ring_ids]]
            if len(seeds) < 100:
                seeds = choose
            sample = q[rng.choice(seeds, min(len(seeds), 1600), replace=False)]
            best = None
            # A seed needs a non-collinear, resolved meridian arc. All
            # candidate supports are fitted again in full 3D and must pass
            # per-triangle geometry and topology checks in the chart stage.
            for _ in range(512):
                pts = sample[rng.choice(len(sample), 3, replace=False)]
                fit = circumcenter(pts)
                if fit is None:
                    continue
                (cx, R), r, _ = fit
                if not R > r > 0.05 or np.ptp(pts[:, 0]) < min(0.1, r / 4):
                    continue
                score = int(
                    (
                        np.abs(np.linalg.norm(sample - [cx, R], axis=1) - r) < 0.0004
                    ).sum()
                )
                if best is None or score > best[0]:
                    best = score, [cx, cy, cz, R, r]
            if best is None or best[0] < 8:
                break
            fitted = refine(p[choose], best[1])
            if fitted is None:
                break
            h, err = fitted
            good = choose[err < 2e-5]
            mask = np.zeros(len(p), dtype=bool)
            mask[good] = True
            triangles = mask[tri].all(axis=1).sum()
            available[good] = False
            if triangles < 100:
                continue
            if not any(np.linalg.norm(h - other) < 0.001 for _, other in hypotheses):
                hypotheses.append((triangles, h))
    return [h.tolist() for _, h in sorted(hypotheses, key=lambda q: -q[0])]


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("witness", type=Path)
    args = parser.parse_args()
    p, t, _ = util["read_source"](args.witness)
    print(json.dumps(find(p, t), indent=2, allow_nan=False))
