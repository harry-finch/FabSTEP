"""Qualify the five source cones identified by the measured bottom-plane loops.

Input: qualify-flange-rim report with analytic seams, and the opt-in R-080 cone
dump. Outputs private measured geometry only to the path passed by the caller.
"""
import argparse
import json
import re
from collections import defaultdict
from pathlib import Path

import numpy as np


RECORD = np.dtype([("triangle", "<u4"), ("cone", "<u4"), ("points", "<f8", (3, 3))])
POINT_TOL = 1e-5  # mm, witness motion for this qualification
BUDGET = 0.05  # mm, default Auto deviation for the source qualification


def require(condition, message):
    if not condition:
        raise ValueError(message)


def rows(path):
    with path.open() as stream:
        for line in stream:
            yield line.rstrip("\n").split("\t")


def owner(label):
    match = re.fullmatch(r"([A-Za-z]+) \{ index: (\d+) \}", label)
    require(match is not None, "Unexpected source owner label")
    return match[1].lower(), int(match[2])


def qualify_cone(samples, base, distal, neighbors, seams):
    require(len(samples) >= 6, "Missing source triangles")
    center = np.array(base["center"])
    delta = np.array(distal["center"]) - center
    axis = np.array(base["axis"])
    height = float(delta @ axis)
    ra, rb = base["radius_mm"], distal["radius_mm"]
    require(np.array_equal(axis, [1., 0., 0.]) and distal["axis"] == base["axis"]
            and np.array_equal(delta[1:], [0., 0.]) and height > POINT_TOL
            and min(ra, rb) > POINT_TOL and abs(ra-rb) > POINT_TOL,
            "Expected two distinct coaxial positive rings along X")
    xyz, inv = np.unique(samples["points"].reshape(-1, 3), axis=0, return_inverse=True)
    faces = inv.reshape(-1, 3)
    rel = xyz - center
    # In this measured domain all vertices lie on one of the two end rings.
    level = (rel[:, 0] > height/2).astype(int)
    radius = np.where(level, rb, ra)
    yz_length = np.linalg.norm(rel[:, 1:], axis=1)
    witness = np.hypot(yz_length-radius, rel[:, 0]-height*level)
    require(np.all(witness <= POINT_TOL), "End-ring witness moved")
    angle = np.arctan2(rel[:, 2], rel[:, 1])
    theta = angle[faces]
    # Lift each short triangle across the periodic seam with a unique branch.
    theta = np.angle(np.exp(1j*(theta-theta[:, :1])))
    span = np.ptp(theta, axis=1)
    require(np.all(span < np.pi/2), "Ambiguous angular lift")
    levels = level[faces]
    turn = (theta[:, 1]*(levels[:, 2]-levels[:, 0])
            - theta[:, 2]*(levels[:, 1]-levels[:, 0]))
    require(np.all(turn < -1e-12), "Fold, collapse or incorrect inner orientation")
    edges = np.stack([faces, np.roll(faces, -1, axis=1)], axis=2).reshape(-1, 2)
    keys = np.min(edges, axis=1)*len(xyz) + np.max(edges, axis=1)
    unique, starts, freq = np.unique(keys, return_index=True, return_counts=True)
    shared = unique[freq == 2]
    require(np.all(freq <= 2) and np.isin(shared, keys[edges[:, 0] < edges[:, 1]]).all()
            and np.isin(shared, keys[edges[:, 0] > edges[:, 1]]).all(),
            "Non-manifold or inconsistently oriented source edges")
    rim_indices = starts[freq == 1]
    outer = edges[rim_indices]
    require(np.all(level[outer[:, 0]] == level[outer[:, 1]]), "Incomplete strip")
    observed = []
    for end in [0, 1]:
        boundary = rim_indices[level[outer[:, 0]] == end]
        a, b = edges[boundary].T
        require(len(a) >= 3 and len(set(a)) == len(a) and set(a) == set(b),
                "Open or repeated boundary")
        # A consistently directed degree-one covering gives exactly one loop.
        arcs = np.angle(np.exp(1j*(angle[b]-angle[a])))
        require(np.all(arcs*(2*end-1) > 0) and
                abs(arcs.sum()-(2*end-1)*2*np.pi) < 1e-9, "Boundary degree")
        for i in boundary:
            require(seams.get((int(samples["triangle"][i//3]), i % 3)) == neighbors[end],
                    "Source neighbor differs from ring-model incidence")
        observed.append(len(boundary))
    # For a barycentric triangle in (angle,level), r is linear in level.
    # Taylor about its weighted angle: quadratic remainder <= R*span²/8
    # (bounded variance), cross term <= range(r)*span/4 (bounded covariance).
    # The covering and opposite end windings make the same map onto the cone.
    bound = witness[faces].max(axis=1) + max(ra, rb)*span**2/8 + abs(ra-rb)*span/4
    require(np.all(bound <= BUDGET), "Continuous cone bound exceeds budget")
    return dict(triangles=len(samples), boundary_edges=observed,
                vertex_motion_mm=float(witness.max()), distance_bound_mm=float(bound.max()))


def qualify(dump, rim):
    rings = {}
    for i, c, a, r in rows(dump/"rings.tsv"):
        rings[int(i)] = dict(index=int(i), center=json.loads(c), axis=json.loads(a), radius_mm=float(r))
    uses = defaultdict(list)
    for i, kind, j in rows(dump/"ring-uses.tsv"):
        uses[int(i)].append((kind, int(j)))
    seams = {}
    for t, e, label in rows(dump/"cone-seams.tsv"):
        key = (int(t), int(e))
        require(key not in seams, "Duplicate seam")
        seams[key] = owner(label)
    source = dump/"cone-source.bin"
    require(0 < source.stat().st_size <= 600000*RECORD.itemsize
            and source.stat().st_size % RECORD.itemsize == 0, "Invalid cone dump length")
    samples = np.fromfile(source, dtype=RECORD)
    require(len(np.unique(samples["triangle"])) == len(samples), "Duplicate source attribution")
    station = min(loop["axial_station_mm"] for loop in rim["boundary_loops"])
    expected = {}
    for hole in rim["bottom_plane"]["holes"]:
        family, = hole["analytic_seams"]
        k, i = owner(family)
        require(k == "cone" and i not in expected and hole["edges"] == hole["analytic_seams"][family],
                "Ambiguous or incomplete source hole")
        expected[i] = hole["edges"]
    require(len(expected) == 5, "Expected five distinct holes")
    cones = []
    for index, a, b, inner in rows(dump/"cones.tsv"):
        i, a, b = int(index), int(a), int(b)
        if i not in expected:
            continue
        require(inner == "true" and abs(rings[a]["center"][0]-station) <= POINT_TOL,
                "Unexpected cone entry")
        neighbors = []
        for end in [a, b]:
            use = uses[end].copy()
            use.remove(("cone", i))
            require(len(use) == 1, "Ring needs exactly two faces")
            neighbors.append(use[0])
        require(neighbors[0][0] == "plane", "Cone entry is not on the base plane")
        proof = qualify_cone(samples[samples["cone"] == i], rings[a], rings[b], neighbors, seams)
        require(proof["boundary_edges"][0] == expected[i], "Source plane/cone seam does not match")
        uses_json = [dict(kind=k, index=j) for k, j in neighbors]
        cones.append(dict(cone_index=i, inner=True, base_ring=rings[a], exit_ring=rings[b],
                          base_neighbor=uses_json[0], exit_neighbor=uses_json[1], **proof))
    require({c["cone_index"] for c in cones} == expected.keys() and len(cones) == len(expected)
            and len({c["base_neighbor"]["index"] for c in cones}) == 1, "Incomplete common base")
    return {"cones": cones}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dump", type=Path)
    parser.add_argument("rim_report", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    result = qualify(args.dump, json.loads(args.rim_report.read_text()))
    args.output.write_text(json.dumps(result, indent=2, allow_nan=False)+"\n")
