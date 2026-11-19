"""Read the small binary STL witnesses in record order for the R-113 bench."""

import math
import struct
from pathlib import Path


def read_stl(path):
    """Return welded vertices and indexed triangles of one closed oriented mesh."""
    record = struct.Struct("<12x9f2x")
    vertices, triangles = [], []
    ids, edges = {}, {}
    with Path(path).open("rb") as source:
        header = source.read(84)
        if len(header) != 84:
            raise ValueError("truncated binary STL header")
        count = struct.unpack_from("<I", header, 80)[0]
        if not 4 <= count <= 10_000:
            raise ValueError("expected a small R-113 witness")
        neighbors = [[] for _ in range(count)]
        for triangle_id in range(count):
            raw = source.read(record.size)
            if len(raw) != record.size:
                raise ValueError("truncated STL triangle")
            coords = record.unpack(raw)
            if not all(map(math.isfinite, coords)):
                raise ValueError("non-finite STL vertex")
            indices = []
            for i in (0, 3, 6):
                point = coords[i : i + 3]
                if point not in ids:
                    ids[point] = len(vertices)
                    vertices.append(point)
                indices.append(ids[point])
            a, b, c = indices
            if len({a, b, c}) != 3:
                raise ValueError("collapsed STL triangle")
            triangles.append((a, b, c))
            for left, right in ((a, b), (b, c), (c, a)):
                key = min(left, right), max(left, right)
                if key not in edges:
                    edges[key] = (left, triangle_id)
                    continue
                reverse = edges[key]
                if reverse is None or reverse[0] != right:
                    raise ValueError("non-manifold or inconsistently oriented edge")
                neighbors[triangle_id].append(reverse[1])
                neighbors[reverse[1]].append(triangle_id)
                edges[key] = None
        if source.read(1):
            raise ValueError("unexpected trailing STL data")
    if any(first is not None for first in edges.values()):
        raise ValueError("open STL boundary")
    visited, pending = {0}, [0]
    while pending:
        for neighbor in neighbors[pending.pop()]:
            if neighbor not in visited:
                visited.add(neighbor)
                pending.append(neighbor)
    if len(visited) != count:
        raise ValueError("expected one connected witness")
    return vertices, triangles
