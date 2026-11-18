"""Match the principal free STEP wire to the actual remaining flange neighbors."""
import argparse
import json
from pathlib import Path

import numpy as np

VERTEX_TOL = 1e-5


def project(edge, points):
    start, end = np.array(edge["start_mm"]), np.array(edge["end_mm"])
    if edge["kind"] == "line":
        vec = end - start
        extent = np.linalg.norm(vec)
        if extent <= VERTEX_TOL:
            raise ValueError("Unresolved contour line")
        direction = vec / extent
        distance = (points - start) @ direction
        t = np.clip(distance, 0, extent)
        ideal = start + t[:, None] * direction
        radius = None
    elif edge["kind"] == "circle":
        center, axis = np.asarray(edge["center_mm"]), np.asarray(edge["axis"])
        radius, angle = edge["radius_mm"], edge["sweep_rad"]
        if radius <= VERTEX_TOL or not 0 < angle < np.pi or abs(np.linalg.norm(axis)-1) > 1e-11:
            raise ValueError("Unsupported circular trim")
        d = (start-center)/radius
        e = np.cross(axis, d)
        if (abs(np.linalg.norm(d)-1)*radius > 5e-8 or abs(np.dot(d, axis))*radius > 5e-8
                or np.linalg.norm(center+radius*(np.cos(angle)*d + np.sin(angle)*e) - end) > 5e-8):
            raise ValueError("Inconsistent analytic arc endpoints")
        # The span is below pi. Lift around its midpoint, then clamp both
        # endpoints so a tiny wrap at a seam cannot select the opposite arc.
        p = points - center
        a = np.arctan2(p @ e, p @ d)
        a = angle/2 + np.angle(np.exp(1j * (a-angle/2)))
        extent = radius*angle
        t = np.clip(a*radius, 0, extent)
        ideal = center + radius*(np.cos(t/radius)[:, None]*d + np.sin(t/radius)[:, None]*e)
    else:
        raise ValueError("Unsupported wire curve")
    motion = np.linalg.norm(ideal-points, axis=1)
    # Include the cost of snapping source chain termini to the true endpoints.
    for station, target in ((0., start), (extent, end)):
        error = np.linalg.norm(points-target, axis=1)
        close = error <= VERTEX_TOL
        t[close], motion[close] = station, error[close]
    return extent, t, motion, radius


def match(source, native, mapping=None):
    points = np.asarray(source["points_mm"])
    neighbors = source["neighbors"]
    if points.shape != (len(neighbors),3) or not 3 <= len(points) <= 4096 or not np.isfinite(points).all():
        raise ValueError("Invalid source contour")
    edges = native["edges"]
    if len(edges) != 20:
        raise ValueError("Incomplete analytic contour")
    incidence = np.zeros(len(points), dtype=int)
    pieces = []
    for edge in edges:
        extent, t, motion, radius = project(edge, points)
        key = (motion <= VERTEX_TOL) & np.roll(motion <= VERTEX_TOL, -1)
        begins = np.flatnonzero(key & ~np.roll(key,1))
        if len(begins) != 1:
            raise ValueError("Analytic edge lacks one connected source chain")
        i = (begins[0] + np.arange(key.sum()+1)) % len(points)
        delta = np.diff(t[i])
        if not ((delta > 0).all() or (delta < 0).all()) or set(t[i[[0,-1]]]) != {0., extent}:
            raise ValueError("Source chain does not cover the complete trim once")
        incidence[key] += 1
        if mapping is not None:
            # Length station in mm follows the curve orientation in the STEP.
            # Reference both directed incidences to allow later splitting at
            # source-owner changes without inferring the opposite triangle.
            mapping.append(dict(native_edge=len(pieces), edges=[
                dict(inside=source["inside_uses"][k], outside=source["outside_uses"][k],
                     neighbor=neighbors[k], start_mm=float(t[k]), end_mm=float(t[(k+1) % len(t)]))
                for k in i[:-1]]))
        drift = motion[i]
        sag = 0. if radius is None else 2*radius*np.sin(delta/(4*radius))**2
        bound = float((np.maximum(drift[:-1],drift[1:]) + sag).max())
        pieces.append(dict(kind=edge["kind"], source_edges=int(key.sum()),
                           neighbor_owners=len({tuple(neighbors[k]) for k in i[:-1]}),
                           distance_bound_mm=bound + 512*np.finfo(float).eps*max(1.,extent,abs(points).max())))
    if not np.all(incidence == 1):
        raise ValueError("Source edge shared twice or unassigned")
    max_error = max(p["distance_bound_mm"] for p in pieces)
    if max_error > .05:
        raise ValueError("Curve deviation exceeds the qualification budget")
    return dict(native_edges=len(pieces), source_edges=len(points), max_distance_bound_mm=max_error,
                neighbor_owners=len(set(map(tuple, neighbors))), pieces=pieces)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("native", type=Path)
    parser.add_argument("--check-negatives", action="store_true")
    parser.add_argument("--map", type=Path, help="Write source triangle incidences and curve stations locally")
    args = parser.parse_args()
    source = json.loads(args.source.read_text())
    native = json.loads(args.native.read_text())
    mapping = [] if args.map else None
    report = match(source, native, mapping)
    if args.check_negatives:
        from copy import deepcopy
        altered = deepcopy(native)
        arc = next(e for e in altered["edges"] if e["kind"] == "circle")
        shift = np.array([.01, 0, 0])
        for field in ("start_mm", "end_mm", "center_mm"):
            arc[field] = np.asarray(arc[field]) + shift
        duplicate = dict(edges=[*native["edges"][:-1], native["edges"][0]])
        reversed_source = deepcopy(source)
        reversed_source["points_mm"][2:4] = reversed(reversed_source["points_mm"][2:4])
        for s, n in [(source, altered), (source, duplicate), (reversed_source, native)]:
            try:
                match(s, n)
            except ValueError:
                continue
            raise AssertionError("Damaged contour accepted")
        report["negative_controls"] = 3
    if args.map:
        args.map.write_text(json.dumps(mapping, separators=(",", ":"), allow_nan=False) + "\n")
    print(json.dumps(report, indent=2, allow_nan=False))
