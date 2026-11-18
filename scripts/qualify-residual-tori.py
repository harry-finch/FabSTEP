"""Offline diagnostic for the R-080 planar-triangle dump (NumPy and SciPy)."""
import argparse
import json
from pathlib import Path

import numpy as np
from scipy.spatial import cKDTree
from scipy.sparse import coo_matrix
from scipy.sparse.csgraph import connected_components

RECORD = np.dtype([
    ("triangle", "<u4"), ("patch", "<u4"), ("owner", "<u4"),
    ("points", "<f8", (3, 3)),
])
TOLERANCE = 1e-5


def circle(points):
    center = points.mean(axis=0)
    delta = points - center
    squared = np.sum(delta * delta, axis=1)
    offset, _, rank, _ = np.linalg.lstsq(2 * delta, squared - squared.mean(), rcond=None)
    if rank < 2:
        return None
    center += offset
    radius = np.linalg.norm(points - center, axis=1).mean()
    return center, radius


def candidate_circles(points):
    """Form hypotheses locally, then refit on all matching points of one section."""
    tree = cKDTree(points)
    candidates = {}
    for indices in tree.query(points, k=15)[1]:
        fit = circle(points[indices])
        if fit is None:
            continue
        center, radius = fit
        for threshold in (0.002, 0.0002, 0.00002):
            match = np.abs(np.linalg.norm(points - center, axis=1) - radius) < threshold
            if match.sum() < 15:
                break
            fit = circle(points[match])
            if fit is None:
                break
            center, radius = fit
        else:
            key = tuple(np.round([*center, radius], 3))
            if key not in candidates or candidates[key][0] < match.sum():
                candidates[key] = (match.sum(), center, radius)
    return [(c, r) for _, c, r in candidates.values()]


def component_sizes(triangles):
    """Count components joined by a shared mesh edge, including isolated triangles."""
    _, vertex = np.unique(triangles.reshape(-1, 3), axis=0, return_inverse=True)
    vertex = vertex.reshape(-1, 3)
    edges = np.sort(vertex[:, [[0, 1], [1, 2], [2, 0]]].reshape(-1, 2), axis=1)
    order = np.lexsort((edges[:, 1], edges[:, 0]))
    paired = np.all(edges[order[1:]] == edges[order[:-1]], axis=1)
    left, right = order[:-1][paired] // 3, order[1:][paired] // 3
    graph = coo_matrix((np.ones(len(left), dtype=bool), (left, right)),
                       shape=(len(triangles), len(triangles)))
    _, labels = connected_components(graph, directed=False)
    return sorted(np.bincount(labels).tolist(), reverse=True)


def measure(rows, bands, visit_support=None):
    triangles = rows["points"]
    all_owners, total = np.unique(rows["owner"], return_counts=True)
    assigned = np.full(len(rows), -1)
    owners_accepted = set()
    spans = []
    for start, end, axial_center in bands:
        minor = max(abs(start - axial_center), abs(end - axial_center))
        xmin, xmax = triangles[:, :, 0].min(axis=1), triangles[:, :, 0].max(axis=1)
        selected = (xmin >= start - TOLERANCE) & (xmax <= end + TOLERANCE) & (xmax - xmin > TOLERANCE)
        ids = np.flatnonzero(selected)
        pts = triangles[ids]
        vertices = np.unique(pts.reshape(-1, 3), axis=0)
        middle = (start + end) / 2
        levels, counts = np.unique(np.round(vertices[:, 0], 5), return_counts=True)
        levels_interior = (abs(levels - middle) < (end - start) / 4) & (counts >= 15)
        if not levels_interior.any():
            raise ValueError("No sampled section inside band")
        level = min(levels[levels_interior], key=lambda x: abs(x - middle))
        section = vertices[np.abs(vertices[:, 0] - level) <= 5.1e-6]
        section_x = section[:, 0].mean()
        radial_offset = np.sqrt(minor**2 - (section_x - axial_center)**2)
        supports = []
        for center, section_radius in candidate_circles(section[:, 1:]):
            rho = np.linalg.norm(pts[:, :, 1:] - center, axis=2)
            for major in [section_radius - radial_offset, section_radius + radial_offset]:
                if major <= minor:
                    continue
                error = np.abs(np.hypot(pts[:, :, 0] - axial_center, rho - major) - minor)
                mask = error.max(axis=1) <= TOLERANCE
                # A single level or narrow strip can fit many unrelated supports.
                if mask.sum() < 100 or np.ptp(pts[mask, :, 0]) < 0.8 * (end - start):
                    continue
                supports.append((mask.sum(), center, major, mask))
        unique = []
        for size, center, major, mask in sorted(supports, key=lambda s: -s[0]):
            if any(np.linalg.norm(center - c) + abs(major - r) < .001 for c, r in unique):
                continue
            unique.append((center, major))
            tag = len(unique)
            idx = ids[mask]
            assigned[idx] = np.where(assigned[idx] == -1, tag, -2)
        whole = set()
        components = []
        centroid_max = 0.0
        over_budget = 0
        for tag, (center, major) in enumerate(unique, 1):
            owner, count = np.unique(rows["owner"][ids[assigned[ids] == tag]], return_counts=True)
            complete = owner[count == total[np.searchsorted(all_owners, owner)]]
            whole.update(complete.tolist())
            if complete.size:
                complete_ids = ids[np.isin(rows["owner"][ids], complete)]
                faces = triangles[complete_ids]
                if visit_support is not None:
                    visit_support(complete_ids, center, major, minor, axial_center)
                components.extend(component_sizes(faces))
                centroids = faces.mean(axis=1)
                radial = np.linalg.norm(centroids[:, 1:] - center, axis=1)
                errors = np.abs(np.hypot(centroids[:, 0] - axial_center, radial - major) - minor)
                over_budget += int(np.count_nonzero(errors > 0.05))
                centroid_max = max(centroid_max, float(errors.max()))
        owners_accepted.update(whole)
        spans.append(dict(band_mm=[start, end], section_mm=float(section_x), supports=len(unique),
                          components=len(components), largest_components_triangles=sorted(components, reverse=True)[:8],
                          max_centroid_error_mm=centroid_max, centroids_over_005mm=over_budget,
                          triangles=int((assigned[ids] > 0).sum()), complete_owners=len(whole)))
    return dict(tolerance_mm=TOLERANCE, bands=spans, complete_owners=len(owners_accepted),
                matched_triangles=int((assigned > 0).sum()), ambiguous_triangles=int((assigned == -2).sum()))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dump", type=Path)
    args = parser.parse_args()
    if args.dump.stat().st_size % RECORD.itemsize or args.dump.stat().st_size > 600000 * RECORD.itemsize:
        parser.error("Invalid triangle dump size")
    rows = np.fromfile(args.dump, dtype=RECORD)
    # Measured X stations of the private Flansch experiment, mm.
    print(json.dumps(measure(rows, [(0, .5, .5), (6.5, 7, 6.5), (7, 9, 9)]), indent=2))
