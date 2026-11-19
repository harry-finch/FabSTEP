"""Measure interrupted toroidal charts on a closed binary STL.

This independent laboratory writes its witnesses in the measured local frame.
The application uses the Rust detector and the native STEP validator.
"""

import argparse
from collections import Counter
import json
from pathlib import Path
import runpy
import struct

import numpy as np

base = Path(__file__).parent
charts = runpy.run_path(str(base / "qualify-flange-residual-tori.py"))
seeds = runpy.run_path(str(base / "qualify-flange-residual-seeds.py"))
require = charts["require"]


def mesh(path):
    length = path.stat().st_size
    require(84 <= length <= 84 + 600000 * 50, "Large STL size limit")
    dtype = np.dtype(
        [("normal", "<f4", (3,)), ("triangle", "<f4", (3, 3)), ("attribute", "<u2")]
    )
    with path.open("rb") as stream:
        stream.read(80)
        n = struct.unpack("<I", stream.read(4))[0]
        require(n > 0 and 84 + n * 50 == length, "Invalid binary STL length")
        pts = np.fromfile(stream, dtype=dtype, count=n)["triangle"].astype(float)
    require(np.isfinite(pts).all(), "Non-finite source triangle")
    p, ids = np.unique(pts.reshape(-1, 3), axis=0, return_inverse=True)
    tri = ids.reshape(-1, 3)
    a, b, twin = charts["edges"](tri, len(p))
    require(
        (twin >= 0).all()
        and np.array_equal(a, b[twin])
        and np.array_equal(b, a[twin])
        and np.array_equal(twin[twin], np.arange(len(twin))),
        "Expected a closed oriented triangle mesh",
    )
    return p, tri, a, b, twin


def principal_axis(p, tri):
    q = p[tri]
    normals = np.cross(q[:, 1] - q[:, 0], q[:, 2] - q[:, 0])
    area = np.linalg.norm(normals, axis=1)
    require(np.all(area > 0), "Degenerate source triangle")
    normals /= area[:, None]
    chosen = np.argmax(np.abs(normals), axis=1)
    normals *= np.sign(normals[np.arange(len(normals)), chosen])[:, None]
    # Large coplanar skins dominate the working axis. Curved and secondary
    # surface normals are not used as evidence of a torus on their own.
    _, ids = np.unique(np.round(normals, 3), axis=0, return_inverse=True)
    group = np.bincount(ids, weights=area).argmax()
    use = ids == group
    axis = np.average(normals[use], axis=0, weights=area[use])
    axis /= np.linalg.norm(axis)
    j = int(np.argmin(np.abs(axis)))
    first = np.eye(3)[j] - axis * axis[j]
    first /= np.linalg.norm(first)
    return np.array([axis, first, np.cross(axis, first)])


def section_centers(p, tri, a, b, twin):
    from scipy.sparse import coo_matrix
    from scipy.sparse.csgraph import connected_components
    from scipy.spatial import cKDTree

    q = p[tri]
    normals = np.cross(q[:, 1] - q[:, 0], q[:, 2] - q[:, 0])
    nx = np.abs(normals[:, 0] / np.linalg.norm(normals, axis=1))
    ii = np.flatnonzero(
        (a < b)
        & (np.abs(p[a, 0] - p[b, 0]) < 5e-6)
        & (np.minimum(nx[np.arange(len(a)) // 3], nx[twin // 3]) < 0.999)
    )
    ca, cb = a[ii], b[ii]
    graph = coo_matrix((np.ones(len(ii)), (ca, cb)), shape=(len(p), len(p))).tocsr()
    graph = graph + graph.T
    degrees = np.diff(graph.indptr)
    # Isolate regular ring chains before fitting. A vertex where a planar
    # sheet branches cannot connect two independent circular sections.
    keep = (degrees[ca] <= 2) & (degrees[cb] <= 2)
    _, comp = connected_components(
        coo_matrix((np.ones(sum(keep)), (ca[keep], cb[keep])), shape=graph.shape),
        directed=False,
    )
    groups = np.split(np.argsort(comp), np.cumsum(np.bincount(comp))[:-1])
    fits = []
    sources = []

    def record(at):
        fit = seeds["circumcenter"](p[at, 1:])
        if fit is None:
            return False
        c, r, dr = fit
        if not r > 0.05 or dr > max(3e-5, 3e-6 * r) or np.ptp(p[at, 0]) > 3e-5:
            return False
        if np.linalg.norm(np.ptp(p[at, 1:], axis=0)) < r * 0.12:
            return False
        fits.append(c)
        sources.append(at)
        return True

    for ids in groups:
        if len(ids) < 8:
            continue
        if record(ids):
            continue
        nodes = set(map(int, ids))
        ends = [i for i in ids if degrees[i] <= 1]
        start = int(ends[0] if len(ends) else ids[0])
        ordered = []
        while start in nodes:
            ordered.append(start)
            nodes.remove(start)
            candidates = [
                x
                for x in graph.indices[graph.indptr[start] : graph.indptr[start + 1]]
                if x in nodes
            ]
            if not candidates:
                break
            start = int(candidates[0])
        for window in (32, 12):
            for i in range(0, len(ordered) - window + 1, window // 2):
                record(np.array(ordered[i : i + window]))
    if not fits:
        return []
    centers = np.array(fits)
    bins = np.floor(centers / 0.005).astype(int)
    keys, inv = np.unique(bins, axis=0, return_inverse=True)
    near = cKDTree(keys).query_pairs(1.5, output_type="ndarray")
    _, grp = connected_components(
        coo_matrix((np.ones(len(near)), near.T), shape=(len(keys), len(keys))),
        directed=False,
    )
    labels = grp[inv]
    sizes = np.bincount(labels)
    candidates = []
    for i in np.argsort(sizes)[::-1][:32]:
        if sizes[i] < 6:
            break
        at = np.flatnonzero(labels == i)
        candidates.append(
            (
                np.median(centers[at], axis=0),
                np.unique(np.concatenate([sources[j] for j in at])),
            )
        )
    return candidates


def hypotheses(p, tri, centers):
    rng = np.random.default_rng(386)
    out = []
    for center, ids in centers:
        cy, cz = center
        q = np.column_stack((p[:, 0], np.hypot(p[:, 1] - cy, p[:, 2] - cz)))
        available = np.ones(len(p), dtype=bool)
        candidates = np.ones(len(ids), dtype=bool)
        for iteration in range(20):
            choose = np.flatnonzero(available)
            sample_ids = ids[candidates & available[ids]]
            if len(choose) < 100 or len(sample_ids) < 80:
                break
            sample = q[
                rng.choice(sample_ids, min(len(sample_ids), 1800), replace=False)
            ]
            best = None
            for _ in range(512):
                pts = sample[rng.choice(len(sample), 3, replace=False)]
                f = seeds["circumcenter"](pts)
                if f is None:
                    continue
                (x, R), r, _ = f
                if not R > r > 0.05 or np.ptp(pts[:, 0]) < min(0.1, r / 4):
                    continue
                err = np.abs(np.linalg.norm(sample - [x, R], axis=1) - r)
                score = int(np.sum(err < 0.0004))
                if best is None or score > best[0]:
                    best = (score, [x, cy, cz, R, r])
            if best is None or best[0] < 8:
                break
            h = best[1]
            fitted = seeds["refine"](p[choose], h)
            if fitted is None:
                bad = (
                    np.abs(np.linalg.norm(q[ids] - [h[0], h[3]], axis=1) - h[4]) < 0.001
                )
                candidates[bad] = False
                continue
            h, err = fitted
            good = choose[err < 2e-5]
            available[good] = False
            mask = np.zeros(len(p), dtype=bool)
            mask[good] = True
            support = int(mask[tri].all(axis=1).sum())
            if support >= 100 and not any(
                np.linalg.norm(h - g) < 0.001 for _, g in out
            ):
                out.append((support, h))
    return [h.tolist() for _, h in sorted(out, key=lambda x: -x[0])]


def unsafe_collars(p, tri, claims, witnesses):
    adjusted = p.copy()
    arcs = {}
    for h, ii, cycles, uv, _, _ in witnesses:
        cx, cy, cz, R, r = h

        def point(u, v):
            rho = R + r * np.cos(v)
            return np.array(
                [cx + r * np.sin(v), cy + rho * np.cos(u), cz + rho * np.sin(u)]
            )

        for loop in cycles:
            for n, (vertex, k) in enumerate(loop):
                other = loop[(n + 1) % len(loop)][0]
                u, v = uv[vertex]
                U, V = uv[other]
                adjusted[vertex] = point(u, v)
                if k == 0:
                    angle = V - v
                    center = np.array([cx, cy + R * np.cos(u), cz + R * np.sin(u)])
                    radius = r
                else:
                    angle = U - u
                    center = np.array([cx + r * np.sin(v), cy, cz])
                    radius = R + r * np.cos(v)
                w = np.cos(angle / 2)
                pole = center + (point((u + U) / 2, (v + V) / 2) - center) / w
                plane_normal = (
                    np.array([0, -np.sin(u), np.cos(u)])
                    if k == 0
                    else np.array([1.0, 0.0, 0.0])
                )
                arcs[tuple(sorted((vertex, other)))] = (
                    pole,
                    w,
                    center,
                    radius,
                    angle,
                    vertex,
                    plane_normal,
                )
    moved = np.linalg.norm(p - adjusted, axis=1) > 0
    hits = np.flatnonzero((claims < 0) & moved[tri].any(axis=1))
    blocked = set()
    classes = Counter()
    examples = []
    for i in hits:
        vs = tri[i]
        src = p[vs]
        dst = adjusted[vs]
        normal = np.cross(src[1] - src[0], src[2] - src[0])
        normal /= np.linalg.norm(normal)
        edges = [arcs.get(tuple(sorted((vs[j], vs[(j + 1) % 3])))) for j in range(3)]
        at = [j for j, q in enumerate(edges) if q is not None]
        if len(at) == 0:
            good = np.cross(dst[1] - dst[0], dst[2] - dst[0]) @ normal > 1e-14
        elif len(at) == 1:
            j = at[0]
            pole, w, *_ = edges[j]
            A = dst[(j + 2) % 3]
            h = np.array([dst[j] - A, (pole - A) * w, dst[(j + 1) % 3] - A])
            d = 2 * np.diff(h, axis=0)
            c = np.zeros(4)
            for l in range(3):
                for k in range(2):
                    c[l + k] += (
                        (1, 2, 1)[l]
                        / (1, 3, 3, 1)[l + k]
                        * (np.cross(h[l], d[k]) @ normal)
                    )
            # For a cone from a point outside the arc plane, each ruling
            # parameter occupies a distinct parallel plane. Its trace there
            # is a scaled injective minor arc even if its projection back
            # onto the source triangle is not injective.
            e = edges[j]
            height = abs(np.dot(A - e[2], e[6]))
            good = c.min() > 1e-14 or (height > 4e-6 and abs(e[4]) < np.pi / 2)
        elif len(at) == 2:
            # A small circular segment with two consecutive arcs is admissible.
            # Otherwise try the projected endpoint Jacobians throughout the
            # curve parameters; the native control-net proof is authoritative.
            j = next(j for j in range(3) if j not in at)
            e1 = edges[(j + 1) % 3]
            e2 = edges[(j + 2) % 3]
            if np.linalg.norm(e1[2] - e2[2]) <= 1e-8 and abs(e1[3] - e2[3]) <= 1e-8:
                good = np.cross(dst[1] - dst[0], dst[2] - dst[0]) @ normal > 1e-14
            else:
                A = dst[(j + 2) % 3]
                B = dst[j]
                C = dst[(j + 1) % 3]

                def curve(t, Q, e):
                    w = e[1]
                    mid = (e[0] - A) * w
                    cap = Q - A
                    N = 2 * t * (1 - t) * mid + t * t * cap
                    W = (1 - t) ** 2 + 2 * w * t * (1 - t) + t * t
                    dN = 2 * (1 - 2 * t) * mid + 2 * t * cap
                    dW = 2 * (1 - 2 * t) * (w - 1)
                    return N / W, (dN * W - N * dW) / (W * W)

                good = True
                for t in (1e-6, 0.25, 0.5, 0.75, 1.0):
                    x, dx = curve(t, B, e2)
                    y, dy = curve(t, C, e1)
                    if (
                        min(np.cross(dx, y - x) @ normal, np.cross(dy, y - x) @ normal)
                        <= 0
                    ):
                        good = False
                        break
        else:
            good = False
        if not good:
            classes[len(at)] += 1
            if len(examples) < 15:
                examples.append(
                    (
                        int(i),
                        len(at),
                        src[0].tolist(),
                        [tuple(np.round(e[2], 3)) for e in edges if e],
                    )
                )
            blocked.update(v for v in vs if moved[v])
    print("refusal classes", classes, "examples", examples[:5], flush=True)
    return blocked


def measure(path, output):
    p, tri, a, b, twin = mesh(path)
    frame = principal_axis(p, tri)
    p = p @ frame.T
    centers = section_centers(p, tri, a, b, twin)
    models = hypotheses(p, tri, centers)
    guard = set()
    for iteration in range(8):
        blocked = np.zeros(len(p), dtype=bool)
        blocked[list(guard)] = True
        claims = np.full(len(tri), -1, dtype=np.int32)
        result = []
        witnesses = []
        for h in models:
            patches = charts["measure"](p, tri, a, b, twin, blocked, h, 8e-5)
            for _, ids, cycles, uv, sign, bound in sorted(
                patches, key=lambda x: -len(x[1])
            ):
                used = np.unique(tri[ids])
                if blocked[used].any():
                    continue
                require((claims[ids] < 0).all(), "Duplicate source triangle")
                claims[ids] = len(result)
                blocked[used] = True
                witnesses.append((h, ids, cycles, uv, sign, bound))
                result.append(
                    dict(
                        torus=h,
                        source_triangles=len(ids),
                        boundary_edges=sum(map(len, cycles)),
                        orientation=sign,
                        bound_mm=bound,
                    )
                )
        failed = unsafe_collars(p, tri, claims, witnesses)
        bad_charts = [
            (i, len(q[1]), sum(v in failed for loop in q[2] for v, k in loop))
            for i, q in enumerate(witnesses)
        ]
        print(
            "chart failures",
            [(i, count, f) for i, count, f in bad_charts if f],
            flush=True,
        )
        print(
            f"collar iteration {iteration+1}: {len(witnesses)} charts, {int((claims>=0).sum())} triangles, {len(failed)} protected vertices",
            flush=True,
        )
        if not failed:
            break
        for _, ids, loops, uv, _, _ in witnesses:
            used = np.unique(tri[ids])
            for loop in loops:
                # Remove a complete adjacent grid band if an isolated
                # staircase would reintroduce a folded two-arc collar.
                for k in (0, 1):
                    levels = np.array(
                        [uv[v, k] for v, axis in loop if axis == k and v in failed]
                    )
                    if len(levels) < 6:
                        continue
                    clusters = np.split(
                        np.sort(levels),
                        np.where(np.diff(np.sort(levels)) > 0.001)[0] + 1,
                    )
                    for run in clusters:
                        if len(run) < 6:
                            continue
                        level = np.median(run)
                        guard.update(used[np.abs(uv[used, k] - level) < 0.012])
        guard.update(failed)
    else:
        raise ValueError("Chart collars exceed the bounded retry count")
    output.mkdir(exist_ok=True, parents=True)
    with (output / "charts.txt").open("w") as stream:
        print(len(witnesses), file=stream)
        for h, ids, cycles, uv, sign, bound in witnesses:
            print(*h, sign, len(ids), len(cycles), "%.17g" % bound, file=stream)
            print(*ids, file=stream)
            for loop in cycles:
                print(len(loop), file=stream)
                for node, k in loop:
                    print(node, k, *["%.17g" % v for v in uv[node]], file=stream)
    with (output / "witness.txt").open("w") as stream:
        q = p[tri] - p[0]
        volume = np.einsum("ij,ij->i", q[:, 0], np.cross(q[:, 1], q[:, 2])).sum() / 6
        area = (
            np.linalg.norm(np.cross(q[:, 1] - q[:, 0], q[:, 2] - q[:, 0]), axis=1).sum()
            / 2
        )
        require(volume > 0, "Inward source solid")
        print(len(p), len(tri), 0, "%.17g" % volume, "%.17g" % area, file=stream)
        np.savetxt(stream, [p.min(axis=0), p.max(axis=0)], fmt="%.17g")
        np.savetxt(stream, p, fmt="%.17g")
        np.savetxt(stream, np.column_stack((np.arange(len(tri)), tri)), fmt="%d")
    claims.astype("<i4").tofile(output / "torus-owner-by-stl-triangle.bin")
    report = dict(
        source_triangles=len(tri),
        measured_vertices=len(p),
        frame=frame.tolist(),
        hypotheses=models,
        claimed_triangles=int((claims >= 0).sum()),
        charts=result,
    )
    (output / "direct-domains.json").write_text(
        json.dumps(report, indent=2, allow_nan=False) + "\n"
    )
    print(
        json.dumps(
            {k: v for k, v in report.items() if k not in ("hypotheses", "charts")}
            | dict(hypotheses=len(models), domains=len(result))
        )
    )


if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("source", type=Path)
    p.add_argument("output", type=Path)
    args = p.parse_args()
    measure(args.source, args.output)
