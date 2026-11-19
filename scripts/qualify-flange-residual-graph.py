"""Match every result face to its source partition using measured anchors and face adjacency."""

import argparse
from collections import Counter
import json
from pathlib import Path
import runpy

import numpy as np
from scipy.spatial import cKDTree

lab = runpy.run_path(str(Path(__file__).with_name("qualify-flange-remainder.py")))
require = lab["require"]


def compress(owners, charts):
    if charts is None:
        return owners
    require(charts.stat().st_size <= 12 * 1024**2, "Oversized chart membership")
    with charts.open() as stream:
        count = int(stream.readline())
        require(0 < count <= 1024, "Invalid chart count")
        assigned = np.zeros(len(owners), dtype=bool)
        first = int(owners.max()) + 1
        for i in range(count):
            fields = stream.readline().split()
            length, loops = map(int, fields[6:8])
            ids = np.fromstring(stream.readline(), sep=" ", dtype=np.int64)
            require(
                len(ids) == length
                and ids.min() >= 0
                and ids.max() < len(owners)
                and len(np.unique(ids)) == length
                and not assigned[ids].any(),
                "Invalid chart membership",
            )
            owners[ids] = first + i
            assigned[ids] = True
            for j in range(loops):
                for _ in range(int(stream.readline())):
                    stream.readline()
        require(not stream.read().strip(), "Trailing chart membership")
    return owners


def source_graph(folder, charts, large=False):
    if folder.is_file():
        public = runpy.run_path(
            str(Path(__file__).with_name("qualify-flange-residual-tori.py"))
        )
        pts, tri, _ = public["read_source"](folder, large)
        _, _, twin = public["edges"](tri, len(pts))
        require((twin >= 0).all(), "Open public source mesh")
        pairs = np.column_stack((np.arange(len(twin)), twin))
        groups = compress(np.arange(len(tri)), charts)
        return graph_from_groups(groups, pts, tri, pairs)
    ends = json.loads((folder / "rail-ends/rail-ends.json").read_text())
    old, nplanes, analytic, hist, groups, names, *_ = lab["attribution"](
        folder / "source",
        folder / "assembly",
        folder / "bores",
        folder / "lettering.txt",
        ends,
    )
    pts, tri, pairs = lab["read_incidence"](folder / "source", old, nplanes, analytic)
    trims = lab["trim_ids"](
        lab["trim_rows"](folder / "trim-triangles.txt"),
        old,
        nplanes,
        hist,
        groups,
        pts,
        tri,
    )
    groups = groups.astype(np.int32)
    first = len(names)
    groups[trims] = np.arange(first, first + len(trims))
    remainder = np.flatnonzero(groups < 0)
    count = first + len(trims)
    groups[remainder] = compress(count + np.arange(len(remainder)), charts)
    return graph_from_groups(groups, pts, tri, pairs)


def graph_from_groups(groups, pts, tri, pairs):
    _, groups = np.unique(groups, return_inverse=True)
    n = groups.max() + 1
    owners = groups[pairs // 3]
    edges = np.unique(np.sort(owners[owners[:, 0] != owners[:, 1]], axis=1), axis=0)
    adj = [set() for _ in range(n)]
    for i, j in edges:
        adj[i].add(int(j))
        adj[j].add(int(i))
    corners = pts[tri]
    area = np.linalg.norm(
        np.cross(corners[:, 1] - corners[:, 0], corners[:, 2] - corners[:, 0]), axis=1
    )
    center = corners.mean(axis=1)
    masses = np.bincount(groups, weights=area)
    centers = np.column_stack(
        [np.bincount(groups, weights=center[:, i] * area) / masses for i in range(3)]
    )
    return groups, adj, centers


def native_graph(path, n):
    require(path.stat().st_size < 40 * 1024**2, "Oversized native graph")
    adj = []
    centers = []
    totals = Counter()
    counts = Counter()
    for line in path.open():
        row = line.split()
        i, k, tri = map(int, row[:3])
        center = list(map(float, row[4:7]))
        neighbors = set(map(int, row[8:]))
        require(
            i == len(adj)
            and len(neighbors) == int(row[7])
            and i not in neighbors
            and all(0 <= x < n for x in neighbors),
            "Invalid native graph",
        )
        adj.append(neighbors)
        centers.append(center)
        totals[k] += tri
        counts[k] += 1
    require(
        len(adj) == n and all(i in adj[j] for i, js in enumerate(adj) for j in js),
        "Native graph is asymmetric or incomplete",
    )
    return adj, np.array(centers), totals, counts


def refine(source, native, anchors):
    # Unique untouched triangle centroids fix the orientation of the graph.
    # Iterative neighbor signatures then identify curved and adjusted faces.
    # Accept only a bijective result preserving every face-to-face incidence.
    n = len(source)
    left = np.zeros(n, dtype=int)
    right = left.copy()
    for token, (i, j) in enumerate(anchors, 1):
        left[i] = right[j] = token
    for depth in range(min(512, 24_000_000 // n)):
        labels = {}

        def update(adj, old):
            out = np.empty_like(old)
            for i, rim in enumerate(adj):
                sig = (int(old[i]), tuple(sorted(int(old[j]) for j in rim)))
                out[i] = labels.setdefault(sig, len(labels))
            return out

        a = update(source, left)
        b = update(native, right)
        require(Counter(a) == Counter(b), "Source and native face incidences differ")
        if len(labels) == n:
            result = np.argsort(b)[a]
            require(
                all(
                    {result[j] for j in neighbors} == native[result[i]]
                    for i, neighbors in enumerate(source)
                ),
                "Face bijection changes adjacency",
            )
            return result, depth + 1
        if len(np.unique(left)) == len(labels):
            raise ValueError(f"{n-len(labels)} indistinguishable faces in the graph")
        left, right = a, b
    raise ValueError("Face graph refinement exceeded its work limit")


def main(folder, path, output, charts, large=False):
    owners, source, centers = source_graph(folder, charts, large)
    native, other, totals, counts = native_graph(path, len(source))
    distances, nearest = cKDTree(other).query(centers, k=2)
    # Existing measured source triangles remote from the adjusted seam retain
    # their exact centroid. Ambiguous or shifted centroids cannot seed a token.
    selected = np.flatnonzero((distances[:, 0] <= 1e-5) & (distances[:, 1] > 1e-4))
    candidates = nearest[selected, 0]
    uses = Counter(candidates)
    anchors = [(int(x), int(j)) for x, j in zip(selected, candidates) if uses[j] == 1]
    result, rounds = refine(source, native, anchors)
    sidecar = result[owners].astype("<u4")
    require(len(np.unique(sidecar)) == len(source), "Incomplete native attribution")
    sidecar.tofile(output)
    print(
        json.dumps(
            dict(
                source_triangles=len(owners),
                result_faces=len(source),
                unique_anchors=len(anchors),
                rounds=rounds,
                mesh_triangles=sum(totals.values()),
                mesh_triangles_by_occt_surface=totals,
                faces_by_occt_surface=counts,
            ),
            indent=2,
        )
    )


if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__)
    for name in ("checkpoint", "graph", "output"):
        p.add_argument(name, type=Path)
    p.add_argument("--charts", type=Path)
    p.add_argument("--large-source", action="store_true")
    a = p.parse_args()
    main(a.checkpoint, a.graph, a.output, a.charts, a.large_source)
