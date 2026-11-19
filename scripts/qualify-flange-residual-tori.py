"""Qualify residual toroidal charts with source-witnessed circular trim edges."""

import argparse
import json
from pathlib import Path

import numpy as np
from scipy.sparse import coo_matrix
from scipy.sparse.csgraph import connected_components


def require(ok, message):
    if not ok:
        raise ValueError(message)


def read_source(witness, large=False):
    limit = 600000 if large else 100000
    require(witness.stat().st_size < (128 if large else 32) * 1024**2, "Oversized mesh witness")
    with witness.open() as f:
        nv, nt, nb = map(int, f.readline().split()[:3])
        require(
            3 <= nv <= 3 * limit and 0 < nt <= limit and nb <= 3 * nt,
            "Invalid source counts",
        )
        f.readline()
        f.readline()
        p = np.loadtxt(f, max_rows=nv)
        ids = np.loadtxt(f, max_rows=nt, dtype=int)
        protected = set()
        for line in f:
            row = line.split()
            protected.update(map(int, row[2 : 2 + int(row[1])]))
    return p, ids[:, 1:], protected


def edges(tri, n):
    a = tri.ravel()
    b = np.roll(tri, -1, axis=1).ravel()
    key = np.minimum(a, b) * n + np.maximum(a, b)
    order = np.argsort(key)
    nxt = np.roll(order, -1)
    pair = key[order] == key[nxt]
    twin = np.full(len(a), -1)
    twin[order[pair]] = nxt[pair]
    twin[nxt[pair]] = order[pair]
    return a, b, twin


def measure(p, tri, a, b, twin, blocked, h, ring_fit_mm=0.0):
    cx, cy, cz, R, r = h
    rho = np.hypot(p[:, 1] - cy, p[:, 2] - cz)
    error = np.abs(np.hypot(p[:, 0] - cx, rho - R) - r)
    uv = np.column_stack(
        (np.arctan2(p[:, 2] - cz, p[:, 1] - cy), np.arctan2(p[:, 0] - cx, rho - R))
    )
    # Keep a cut between charts crossing the parameter seam. Its source
    # triangles remain available as the external measured collar.
    uv = (uv + 0.075) % (2 * np.pi) - 0.075
    span = np.ptp(uv[tri], axis=1)
    hessian = (
        (R + r) * span[:, 0] ** 2
        + r * span[:, 1] ** 2
        + 2 * r * span[:, 0] * span[:, 1]
    ) / 2
    ids = (
        (error[tri].max(axis=1) < 2e-5) & (hessian < 0.04) & (~blocked[tri].any(axis=1))
    )
    ids &= (span < np.pi / 2).all(axis=1)
    constant = np.abs(uv[a] - uv[b]) < np.maximum(1.5e-5, ring_fit_mm / np.array([R-r,r]))
    iso = constant.sum(axis=1) == 1
    # An exposed diagonal cannot describe a true circle on the support.
    # Peel such triangles until every retained boundary edge is isoparametric.
    for _ in range(1000):
        rim = ids[np.arange(len(a)) // 3] & ((twin < 0) | (~ids[twin // 3]))
        remove = np.unique(np.flatnonzero(rim & ~iso) // 3)
        if not len(remove):
            break
        ids[remove] = False
    else:
        raise ValueError("Excessive chart peeling")
    aa = np.flatnonzero(ids)
    at = np.flatnonzero(ids[np.arange(len(a)) // 3] & (twin >= 0) & ids[twin // 3])
    if not len(aa):
        return []
    count, label = connected_components(
        coo_matrix(
            (
                np.ones(len(at)),
                (np.searchsorted(aa, at // 3), np.searchsorted(aa, twin[at] // 3)),
            ),
            shape=(len(aa), len(aa)),
        ),
        directed=False,
    )
    out = []
    for lab in range(count):
        use = aa[label == lab]
        if len(use) < 30:
            continue
        rim = np.flatnonzero(
            np.repeat(np.isin(np.arange(len(tri)), use), 3)
            & ((twin < 0) | (~ids[twin // 3]))
        )
        paths = {}
        for edge in rim:
            if a[edge] in paths:
                break
            paths[a[edge]] = (b[edge], constant[edge].argmax())
        if len(paths) != len(rim) or set(paths) != {v for v, k in paths.values()}:
            continue
        cycles = []
        while paths:
            start = next(iter(paths))
            node = start
            loop = []
            while node in paths:
                other, k = paths.pop(node)
                loop.append((int(node), int(k)))
                node = other
            require(node == start, "Open measured trim cycle")
            cycles.append(loop)
        snapped = uv.copy()
        valid = True
        for loop in cycles:
            corner = next(
                (i for i, (_, q) in enumerate(loop) if q != loop[i - 1][1]), None
            )
            if corner is None:
                valid = False
                break
            shifted = loop[corner:] + loop[:corner]
            runs = []
            for i, (v, k) in enumerate(shifted):
                if not runs or runs[-1][0] != k:
                    runs.append((k, []))
                runs[-1][1].append(v)
            for k, nodes in runs:
                last = shifted[
                    (next(i for i, (v, _) in enumerate(shifted) if v == nodes[-1]) + 1)
                    % len(shifted)
                ][0]
                verts = nodes + [last]
                values = uv[verts, k]
                if np.ptp(values) * (R + r if k == 0 else r) > 0.04:
                    valid = False
                    break
                snapped[verts, k] = values.mean()
        if not valid:
            continue
        q = snapped[tri[use]]
        det = (q[:, 1, 0] - q[:, 0, 0]) * (q[:, 2, 1] - q[:, 0, 1]) - (
            q[:, 1, 1] - q[:, 0, 1]
        ) * (q[:, 2, 0] - q[:, 0, 0])
        if not (np.all(det > 1e-12) or np.all(det < -1e-12)):
            continue
        # PL triangles cover their uniquely oriented simple boundary if every
        # triangle Jacobian has the same sign. Reject intersecting boundaries.
        segments = [
            (
                snapped[node],
                snapped[loop[(i + 1) % len(loop)][0]],
                node,
                loop[(i + 1) % len(loop)][0],
            )
            for loop in cycles
            for i, (node, k) in enumerate(loop)
        ]
        seg = np.array([[x, y] for x, y, *_ in segments])
        lo = seg.min(axis=1)
        hi = seg.max(axis=1)
        for i, (_, _, a0, b0) in enumerate(segments):
            candidates = (
                np.flatnonzero(
                    (lo[i] <= hi[i + 1 :] + 1e-12).all(axis=1)
                    & (lo[i + 1 :] <= hi[i] + 1e-12).all(axis=1)
                )
                + i
                + 1
            )
            if any(not {a0, b0}.intersection(segments[j][2:]) for j in candidates):
                valid = False
                break
        if not valid:
            continue
        vertices = np.unique(tri[use])
        u, v = snapped[vertices].T
        actual = np.column_stack(
            (
                cx + r * np.sin(v),
                cy + (R + r * np.cos(v)) * np.cos(u),
                cz + (R + r * np.cos(v)) * np.sin(u),
            )
        )
        shift = np.linalg.norm(actual - p[vertices], axis=1).max()
        span = np.ptp(q, axis=1)
        bound = shift + np.max(
            (
                (R + r) * span[:, 0] ** 2
                + r * span[:, 1] ** 2
                + 2 * r * span[:, 0] * span[:, 1]
            )
            / 2
        )
        if not bound < 0.05:
            continue
        # The orientation of the outer cycle is the source-face orientation.
        areas = [
            sum(
                snapped[x, 0] * snapped[loop[(i + 1) % len(loop)][0], 1]
                - snapped[x, 1] * snapped[loop[(i + 1) % len(loop)][0], 0]
                for i, (x, _) in enumerate(loop)
            )
            for loop in cycles
        ]
        sign = 1 if det[0] > 0 else -1
        outside = [i for i, x in enumerate(areas) if x * sign > 0]
        if len(outside) != 1:
            continue
        i = outside[0]
        cycles[0], cycles[i] = cycles[i], cycles[0]
        out.append((h, use, cycles, snapped, sign, bound))
    return out


def prepare(witness, out, hypotheses):
    p, tri, protected = read_source(witness)
    a, b, twin = edges(tri, len(p))
    blocked = np.zeros(len(p), dtype=bool)
    blocked[list(protected)] = True
    patches = []
    for h in hypotheses:
        candidates = measure(p, tri, a, b, twin, blocked, h)
        for patch in sorted(candidates, key=lambda q: -len(q[1])):
            pts = np.unique(tri[patch[1]])
            if blocked[pts].any():
                continue
            patches.append(patch)
            blocked[pts] = True
    with out.open("w") as stream:
        print(len(patches), file=stream)
        for h, ids, cycles, uv, sign, bound in patches:
            print(*h, sign, len(ids), len(cycles), "%.17g" % bound, file=stream)
            print(*ids, file=stream)
            for loop in cycles:
                print(len(loop), file=stream)
                for node, k in loop:
                    print(node, k, *["%.17g" % v for v in uv[node]], file=stream)
    return dict(
        charts=len(patches),
        source_triangles=sum(len(q[1]) for q in patches),
        trim_edges=sum(len(loop) for q in patches for loop in q[2]),
        largest_charts=sorted((len(q[1]) for q in patches), reverse=True)[:12],
        worst_certified_mm=max((q[-1] for q in patches), default=None),
    )


if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("witness", type=Path)
    p.add_argument("hypotheses", type=Path)
    p.add_argument("output", type=Path)
    args = p.parse_args()
    hypotheses = json.loads(args.hypotheses.read_text())
    require(
        0 < len(hypotheses) <= 16
        and all(
            len(h) == 5 and np.isfinite(h).all() and h[3] > h[4] > 0 for h in hypotheses
        ),
        "Invalid torus hypotheses",
    )
    print(json.dumps(prepare(args.witness, args.output, hypotheses), indent=2))
