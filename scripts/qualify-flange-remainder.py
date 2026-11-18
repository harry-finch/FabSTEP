"""Reconcile the local Flansch shell, source ownership and remaining mesh interfaces."""

import argparse
from collections import Counter, defaultdict
import json
from pathlib import Path
import runpy

import numpy as np
from scipy.sparse import coo_matrix
from scipy.sparse.csgraph import connected_components
from scipy.spatial import cKDTree

root = Path(__file__).parent
ownership = runpy.run_path(str(root / "qualify-flange-ownership.py"))
bores = runpy.run_path(str(root / "qualify-flange-bores.py"))
require = ownership["require"]
plane_row = np.dtype(
    [("triangle", "<u4"), ("patch", "<u4"), ("owner", "<u4"), ("points", "<f8", (3, 3))]
)


def marked_owners(path):
    require(path.stat().st_size < 8 * 1024**2, "Oversize lettering witness")
    with path.open() as stream:
        vertices, faces = map(int, next(stream).split())
        require(0 < vertices <= 16000 and faces == 1752, "Unexpected lettering size")
        for _ in range(vertices):
            next(stream)
        owners = []
        for _ in range(faces):
            owner, _, _, _, _, n = next(stream).split()
            require(0 < int(n) <= 32, "Invalid lettering wire count")
            owners.append(int(owner))
            for _ in range(int(n)):
                next(stream)
        require(
            not stream.read().strip() and len(set(owners)) == faces,
            "Repeated lettering owner",
        )
    return owners


def rail_end_owners(ends, old, nplanes):
    require(isinstance(ends, list) and len(ends) == 2, "Expected two rail ends")
    ids = [face["owner"] for face in ends]
    require(len(set(ids)) == 2, "Duplicate rail end")
    for owner, face in zip(ids, ends, strict=True):
        require(type(owner) is int and 0 <= owner < nplanes, "Unknown rail end owner")
        triangles = face["triangles"]
        require(
            len(triangles) == 2
            and np.array_equal(np.sort(triangles), np.flatnonzero(old == owner)),
            "Rail end does not cover its full measured owner",
        )
    return ids


def attribution(dump, assembly, chains, markings, ends=None):
    old, nplanes, analytic, hist = ownership["automatic_owners"](dump)
    counts = json.loads((assembly / "counts.json").read_text())
    names = list(counts)
    require(
        set(names) == set(ownership["LABELS"] + [f"long-flank-{i}" for i in (4, 5, 6)]),
        "Expected qualified 37-face assignment",
    )
    with (assembly / "patches.tsv").open() as stream:
        groups = ownership["load_groups"](stream, len(old), names)
    ownership["audit"](old, nplanes, analytic, hist, groups, counts, names)
    chain_data = json.loads((chains / "bores.json").read_text())
    chain_counts = {
        f"bore-{c['kind']}-{c['index']}": c["triangles"] for c in chain_data
    }
    require(len(chain_data) == len(chain_counts) == 22, "Unexpected bore chain count")
    with (chains / "bores.tsv").open() as stream:
        extras = ownership["load_groups"](stream, len(old), list(chain_counts))
    claimed = extras >= 0
    require(np.all(groups[claimed] == -1), "A bore chain overlaps a previous claim")
    groups[claimed] = len(names) + extras[claimed]
    for j, face in enumerate(chain_data):
        index = face["index"]
        if face["kind"] != "plane":
            index = nplanes + analytic.index(
                f"{face['kind'].title()} {{ index: {index} }}"
            )
        require(
            np.all(old[extras == j] == index),
            "Bore chain does not own its measured source face",
        )
    names.extend(chain_counts)
    counts.update(chain_counts)
    additional = [41637, 3, 21957, *marked_owners(markings)]
    if ends is not None:
        additional.extend(rail_end_owners(ends, old, nplanes))
    require(
        len(set(additional)) == len(additional)
        and all(0 <= i < nplanes for i in additional),
        "Overlapping additional planes",
    )
    lookup = np.full(len(hist), -1, dtype=groups.dtype)
    for index in additional:
        lookup[index] = len(names)
        name = f"plane-{index}"
        names.append(name)
        counts[name] = int(hist[index])
    extra = lookup[old]
    require(
        np.all(groups[extra >= 0] == -1), "An additional plane overlaps a prior claim"
    )
    groups[extra >= 0] = extra[extra >= 0]
    summary = checked_partition(old, nplanes, hist, groups, names, counts)
    return old, nplanes, analytic, hist, groups, names, counts, summary


def checked_partition(old, nplanes, hist, groups, names, counts):
    lo = np.full(len(hist), len(names), dtype=groups.dtype)
    hi = np.full(len(hist), -1, dtype=groups.dtype)
    np.minimum.at(lo, old, groups)
    np.maximum.at(hi, old, groups)
    require(np.array_equal(lo, hi), "An automatic owner was split")
    require(np.all(lo[nplanes:] >= 0), "Unclaimed non-planar owner")
    used = groups >= 0
    actual = np.bincount(groups[used], minlength=len(names))
    require(
        np.array_equal(actual, [counts[k] for k in names]),
        "Incorrect measured face counts",
    )
    final = np.where(used, groups.astype(np.int64) + len(hist), old)
    ids, sizes = np.unique(final, return_counts=True)
    require(
        np.array_equal(
            ids, np.r_[np.flatnonzero(lo < 0), len(hist) + np.arange(len(names))]
        )
        and np.array_equal(sizes, np.r_[hist[lo < 0], actual])
        and sizes.sum() == len(old),
        "Incomplete final owner partition",
    )
    summary = dict(
        source_triangles=len(old),
        original_faces=len(hist),
        shell_faces=len(names),
        shell_triangles=int(used.sum()),
        replaced_owners=int((lo >= 0).sum()),
        remaining_triangles=int((~used).sum()),
        remaining_planar_owners=int((lo < 0).sum()),
        hypothetical_faces=len(ids),
        over_100000=max(0, len(ids) - 100000),
    )
    return summary


def read_incidence(dump, old, nplanes, analytic):
    # Read by analysis triangle ID. Geometric welding here is exact equality of
    # the analysis vertices; no proximity operation can create source adjacency.
    points = np.empty((len(old), 3, 3))
    seen = np.zeros(len(old), dtype=bool)
    formats = [
        ("planes-3deg.bin", plane_row),
        ("cone-source.bin", bores["cone"]["RECORD"]),
        ("bore-source.bin", bores["RECORD"]),
    ]
    for filename, dtype in formats:
        rows = bores["records"](dump / filename, dtype)
        ids = rows["triangle"]
        require(
            ids.max() < len(old) and not seen[ids].any(),
            "Duplicate or invalid source dump index",
        )
        if filename.startswith("planes"):
            source_owner = rows["owner"]
        else:
            if filename.startswith("cone"):
                indices, kinds = rows["cone"], np.zeros(len(rows), dtype=int)
                kind_names = {0: "Cone"}
            else:
                indices, kinds = rows["index"], rows["kind"]
                kind_names = {0: "Cylinder", 1: "Torus"}
            source_owner = np.empty(len(rows), dtype=int)
            for kind, index in np.unique(np.column_stack([kinds, indices]), axis=0):
                name = f"{kind_names[kind]} {{ index: {index} }}"
                source_owner[(kinds == kind) & (indices == index)] = (
                    nplanes + analytic.index(name)
                )
        require(
            np.array_equal(old[ids], source_owner),
            "Geometry and ownership rows disagree",
        )
        seen[ids] = True
        points[ids] = rows["points"]
    require(
        seen.all() and np.isfinite(points).all(), "Missing or nonfinite source geometry"
    )
    vertices, inverse = np.unique(points.reshape(-1, 3), axis=0, return_inverse=True)
    triangles = inverse.reshape(-1, 3)
    a = triangles.ravel()
    b = np.roll(triangles, -1, axis=1).ravel()
    keys = np.minimum(a, b) * len(vertices) + np.maximum(a, b)
    pairs = np.argsort(keys).reshape(-1, 2)
    left, right = pairs.T
    require(
        np.all(keys[left] == keys[right])
        and np.all(np.diff(keys[left]) > 0)
        and np.array_equal(a[left], b[right])
        and np.array_equal(b[left], a[right])
        and np.all(a[left] != b[left]),
        "Full source mesh lacks opposite manifold incidence",
    )
    return vertices, triangles, pairs


def components(n, a, b):
    adjacency = coo_matrix((np.ones(len(a), dtype=bool), (a, b)), shape=(n, n)).tocsr()
    return connected_components(adjacency, directed=False)


def trim_rows(path):
    require(0 < path.stat().st_size < 4 * 1024**2, "Invalid trim manifest size")
    with path.open() as stream:
        count = int(next(stream))
        rows = np.loadtxt(stream, ndmin=2)
    require(
        count == 267 and rows.shape == (count, 22) and np.isfinite(rows).all(),
        "Incomplete trim manifest",
    )
    return rows


def trim_ids(rows, old, nplanes, hist, groups, vertices, triangles):
    owners = rows[:, 0].astype(int)
    require(
        np.array_equal(owners, rows[:, 0])
        and len(np.unique(owners)) == len(owners)
        and np.all((0 <= owners) & (owners < nplanes)),
        "Invalid or duplicate trim owner",
    )
    require(np.all(hist[owners] == 1), "A trim owner spans several triangles")
    ids = np.flatnonzero(np.isin(old, owners))
    require(
        len(ids) == len(owners) and np.all(groups[ids] < 0),
        "A trim owner is already claimed",
    )
    # The manifest stores raw corners as native edge direction followed by
    # apex. This reverses the source triangle around a cyclic corner.
    by_owner = ids[np.argsort(old[ids])]
    in_file_order = by_owner[np.searchsorted(old[by_owner], owners)]
    source = vertices[triangles[in_file_order]]
    raw = rows[:, -9:].reshape(-1, 3, 3)[:, [1, 0, 2], :]
    same = np.array(
        [np.all(raw == np.roll(source, i, axis=1), axis=(1, 2)) for i in range(3)]
    )
    require(
        np.all(same.sum(axis=0) == 1),
        "Trim triangle geometry or winding differs from its owner",
    )
    return ids


def trim_neighbors(path, ids, old, groups, vertices, triangles, pairs):
    opposite = np.empty(triangles.size, dtype=int)
    left, right = pairs.T
    opposite[left] = right
    opposite[right] = left
    selected = np.zeros(len(old), dtype=bool)
    selected[ids] = True
    counts = Counter()
    with path.open("w") as stream:
        print(3 * len(ids), file=stream)
        for i in ids:
            for k in range(3):
                neighbor = opposite[3 * i + k] // 3
                tag = (
                    "existing"
                    if groups[neighbor] >= 0
                    else "added"
                    if selected[neighbor]
                    else "free"
                )
                counts[tag] += 1
                print(
                    old[i],
                    k,
                    tag,
                    old[neighbor],
                    *vertices[triangles[i, k]],
                    *vertices[triangles[i, (k + 1) % 3]],
                    file=stream,
                )
    return dict(counts)


def interface_report(vertices, triangles, pairs, groups, old, names, source_path=None):
    left, right = pairs.T // 3
    remaining = groups < 0
    ids = np.flatnonzero(remaining)
    inside = remaining[left] & remaining[right]
    count, labels = components(
        len(ids),
        np.searchsorted(ids, left[inside]),
        np.searchsorted(ids, right[inside]),
    )
    report = []
    for label in range(count):
        at = ids[labels == label]
        report.append(
            dict(
                triangles=len(at),
                owners=len(np.unique(old[at])),
                owner_ids=np.unique(old[at]).tolist() if len(at) <= 10 else None,
                first_analysis_triangle=int(at.min()),
            )
        )
    crosses = remaining[left] != remaining[right]
    both = pairs[crosses]
    # Orient every interface edge as on its residual source triangle.
    side, other = np.where(remaining[both[:, :1] // 3], both, both[:, ::-1]).T
    src = triangles[side // 3, side % 3]
    dst = triangles[side // 3, (side + 1) % 3]
    unique, node = np.unique(np.r_[src, dst], return_inverse=True)
    a, b = node.reshape(2, -1)
    num, network = components(len(unique), a, b)
    indegree = np.bincount(b, minlength=len(unique))
    outdegree = np.bincount(a, minlength=len(unique))
    require(np.array_equal(indegree, outdegree), "Unbalanced source interfaces")
    systems = []
    for net in range(num):
        nodes = np.flatnonzero(network == net)
        mask = network[a] == net
        degree = (indegree + outdegree)[nodes]
        systems.append(
            dict(
                edges=int(mask.sum()),
                vertices=len(nodes),
                cycle_rank=int(mask.sum() - len(nodes) + 1),
                vertex_degrees=dict(sorted(Counter(map(int, degree)).items())),
                branch_points_mm=vertices[unique[nodes[degree != 2]]].tolist(),
                remainder_components=np.unique(
                    labels[np.searchsorted(ids, side[mask] // 3)]
                ).tolist(),
                shell_neighbors=dict(
                    Counter(names[i] for i in groups[other[mask] // 3])
                ),
            )
        )
    triangles_left = labels[np.searchsorted(ids, side // 3)]
    for i, part in enumerate(report):
        mask = triangles_left == i
        nodes, arcs = np.unique(np.r_[a[mask], b[mask]], return_inverse=True)
        src_part, dst_part = arcs.reshape(2, -1)
        degree = np.bincount(src_part, minlength=len(nodes)) + np.bincount(
            dst_part, minlength=len(nodes)
        )
        require(
            np.all(degree == 2),
            "A residual component has a nonsimple directed boundary",
        )
        number, contours = components(len(nodes), src_part, dst_part)
        part["contour_edges"] = sorted(
            int((contours[src_part] == k).sum()) for k in range(number)
        )
    require(
        sum(part["owners"] for part in report) == len(np.unique(old[remaining])),
        "A remaining planar owner spans disconnected triangle components",
    )
    frontier = [
        dict(
            start_mm=vertices[i].tolist(),
            end_mm=vertices[j].tolist(),
            network=int(net),
            component=int(comp),
            remaining_triangle=int(s // 3),
            remaining_edge=int(s % 3),
            remaining_owner=int(old[s // 3]),
            measured_triangle=int(o // 3),
            measured_edge=int(o % 3),
            measured_owner=int(old[o // 3]),
            measured_group=names[groups[o // 3]],
        )
        for i, j, net, comp, s, o in zip(
            src, dst, network[a], triangles_left, side, other, strict=True
        )
    ]
    if source_path:
        source_path.write_text(
            json.dumps(dict(edges=frontier), separators=(",", ":"), allow_nan=False)
            + "\n"
        )
    return (
        dict(residual_edge_components=report, boundary_networks=systems),
        vertices[unique],
        a,
        b,
        frontier,
    )


def circle_bound(edge, points):
    axis = np.asarray(edge["axis"])
    x = np.asarray(edge["zero_direction"])
    y = np.cross(axis, x)
    delta = points - edge["center_mm"]
    theta = np.unwrap(np.arctan2(delta @ y, delta @ x))
    theta -= 2 * np.pi * round((theta[0] - edge["start_rad"]) / (2 * np.pi))
    start = edge["start_rad"]
    end = start + edge["signed_sweep_rad"]
    theta[[0, -1]] = start, end
    require(
        np.all((end - start) * np.diff(theta) > 0),
        "Circle does not follow the measured chain",
    )
    radius = edge["radius_mm"]
    projected = edge["center_mm"] + radius * (
        np.cos(theta)[:, None] * x + np.sin(theta)[:, None] * y
    )
    shift = np.linalg.norm(points - projected, axis=1)
    sag = radius * np.diff(theta) ** 2 / 8
    bound = float(np.max(np.maximum(shift[:-1], shift[1:]) + sag))
    require(bound <= 0.05, "A matched source chord exceeds its circle tolerance")
    return bound


def match_native(native, xyz, a, b, frontier):
    tree = cKDTree(xyz)
    lookup = {(int(v), int(u)): i for i, (u, v) in enumerate(zip(a, b, strict=True))}
    require(len(lookup) == len(a), "Duplicate directed source boundary edge")
    seen = set()
    loops = []
    loop_visits = defaultdict(list)
    max_end = max_sample = max_circle = 0.0
    for loop in native["loops"]:
        edges = loop["edges"]
        coordinates = np.asarray([[e["start_mm"], e["end_mm"]] for e in edges])
        distances, node = tree.query(coordinates, k=2)
        max_end = max(max_end, float(distances[:, :, 0].max()))
        require(
            max_end <= 1e-5 and distances[:, :, 1].min() > 1e-5,
            "Ambiguous native/source endpoint",
        )
        pairs = node[:, :, 0]
        require(
            np.array_equal(pairs[:, 1], np.roll(pairs[:, 0], -1)),
            "Native frontier is not cyclic",
        )
        paths = []
        associations = defaultdict(set)
        for i, u in enumerate(pairs[:, 0]):
            loop_visits[int(u)].append((loop["id"], i, coordinates[i, 0]))
        for edge, (u, v) in zip(edges, pairs, strict=True):
            row = lookup.get((u, v))
            if row is None:
                paths.append([])
                continue
            require(row not in seen, "Repeated native use of source edge")
            seen.add(row)
            associations[edge["face_id"]].add(frontier[row]["measured_group"])
            paths.append([row])
        multi = []
        for i, (u, v) in enumerate(pairs):
            edge = edges[i]
            if not paths[i]:
                # After both end planes are present a neighboring upper tube
                # has only a single unsplit quarter circle on the frontier.
                # Identify its measured group at both ends, where the other
                # source chains have already paired with individual edges.
                entering = {
                    frontier[j]["measured_group"]
                    for j in np.flatnonzero(b == u)
                    if j not in seen
                }
                exiting = {
                    frontier[j]["measured_group"]
                    for j in np.flatnonzero(a == v)
                    if j not in seen
                }
                possible = entering & exiting
                known = associations[edge["face_id"]]
                if known:
                    possible &= known
                require(len(possible) == 1, "Native edge lacks a unique source support")
                (group,) = possible
                associations[edge["face_id"]].add(group)
                # All remaining uses of this support are measured arc segments.
                require(
                    edge["kind"] == "circle", "Unexpected many-to-one noncircular edge"
                )
                cursor = u
                while cursor != v:
                    candidates = [
                        j
                        for j in np.flatnonzero(b == cursor)
                        if j not in seen and frontier[j]["measured_group"] == group
                    ]
                    require(
                        len(candidates) == 1,
                        "Broken or branched source chain on native support",
                    )
                    j = candidates[0]
                    seen.add(j)
                    paths[i].append(j)
                    cursor = a[j]
                multi.append(
                    dict(
                        face_id=edge["face_id"],
                        measured_group=group,
                        source_edges=len(paths[i]),
                    )
                )
            path = paths[i]
            points = xyz[np.r_[b[path[0]], a[path]]]
            if edge["kind"] == "circle":
                max_circle = max(max_circle, circle_bound(edge, points))
            else:
                require(
                    len(path) == 1 and edge["kind"] in ("line", "bspline"),
                    "Unknown native edge support",
                )
                samples = np.asarray(edge["samples_mm"])
                # A sampled discrepancy, not a continuous bound on a spline.
                fractions = np.linspace(0.0, 1.0, len(samples))[:, None]
                error = np.linalg.norm(
                    samples - ((1 - fractions) * points[0] + fractions * points[-1]),
                    axis=1,
                ).max()
                require(
                    error <= 0.05, "Native curve samples differ from their source chord"
                )
                max_sample = max(max_sample, float(error))
        require(
            all(len(groups) == 1 for groups in associations.values()),
            "Inconsistent native face ownership",
        )
        src = [index for chain in paths for index in chain]
        # A STEP cycle with more than one occurrence of a source vertex is
        # topologically simple in OCCT but touches itself in the measured graph.
        vertices = Counter(int(b[j]) for j in src)
        repeated = []
        for node, visits in vertices.items():
            if visits <= 1:
                continue
            # The repeated nodes here are endpoints of native edges, not just
            # samples inside an analytic arc. Keep both native occurrences.
            indices = np.flatnonzero(pairs[:, 0] == node)
            require(
                len(indices) == visits, "A multi-contact lies inside an aggregated trim"
            )
            positions = coordinates[indices, 0]
            repeated.append(
                dict(
                    source_mm=xyz[node].tolist(),
                    native_edge_starts=indices.tolist(),
                    max_pair_separation_mm=float(
                        np.linalg.norm(
                            positions[:, None] - positions[None, :], axis=2
                        ).max()
                    ),
                )
            )
        loops.append(
            dict(
                native_loop=loop["id"],
                native_edges=len(edges),
                opposite_source_edges=len(src),
                one_to_one=sum(len(chain) == 1 for chain in paths),
                aggregated_arcs=multi,
                source_networks=sorted({frontier[j]["network"] for j in src}),
                repeated_source_vertices=len(repeated),
                repeated_occurrences=repeated,
                components=dict(Counter(frontier[j]["component"] for j in src)),
            )
        )
    require(
        len(seen) == len(frontier)
        and sum(v["native_edges"] for v in loops) == native["free_edges"],
        "Incomplete source/native coverage",
    )
    report = dict(
        loops=loops,
        max_endpoint_motion_mm=max_end,
        max_line_spline_sample_error_mm=max_sample,
        max_circle_chord_bound_mm=max_circle,
    )
    contacts = []
    for node, occurrence in loop_visits.items():
        if len({loop for loop, _, _ in occurrence}) <= 1:
            continue
        positions = np.asarray([p for _, _, p in occurrence])
        contacts.append(
            dict(
                source_mm=xyz[node].tolist(),
                native_loop_edges=[(i, j) for i, j, _ in occurrence],
                max_pair_separation_mm=float(
                    np.linalg.norm(
                        positions[:, None] - positions[None, :], axis=2
                    ).max()
                ),
            )
        )
    if contacts:
        report["source_contacts_between_native_loops"] = contacts
    return report


def refused(action, expected):
    try:
        action()
    except ValueError as error:
        if str(error) == expected:
            return
        raise
    raise AssertionError("Negative control accepted: " + expected)


def trim_refusals(rows, old, nplanes, hist, groups, vertices, triangles):
    args = (old, nplanes, hist, groups, vertices, triangles)
    wrong = rows.copy()
    wrong[0] = wrong[1]
    refused(lambda: trim_ids(wrong, *args), "Invalid or duplicate trim owner")
    wrong = rows.copy()
    wrong[0, 0] = np.flatnonzero(hist[:nplanes] > 1)[0]
    refused(lambda: trim_ids(wrong, *args), "A trim owner spans several triangles")
    wrong[0, 0] = old[
        np.flatnonzero((groups >= 0) & (old < nplanes) & (hist[old] == 1))[0]
    ]
    refused(lambda: trim_ids(wrong, *args), "A trim owner is already claimed")
    wrong = rows.copy()
    wrong[0, -1] += 0.1
    refused(
        lambda: trim_ids(wrong, *args),
        "Trim triangle geometry or winding differs from its owner",
    )
    return [
        "repeated_trim_owner",
        "multi_triangle_trim_owner",
        "already_claimed_trim_owner",
        "wrong_trim_vertex",
    ]


def refusal_controls(
    old, nplanes, hist, groups, names, counts, native, match_args, ends
):
    row = f"0\t{names[0]}\n"
    refused(
        lambda: ownership["load_groups"]([row, row], len(old), names),
        "Repeated triangle assignment",
    )
    partial = groups.copy()
    partial[np.flatnonzero((partial >= 0) & (hist[old] > 1))[0]] = -1
    refused(
        lambda: checked_partition(old, nplanes, hist, partial, names, counts),
        "An automatic owner was split",
    )
    incomplete = groups.copy()
    incomplete[old == nplanes] = -1
    refused(
        lambda: checked_partition(old, nplanes, hist, incomplete, names, counts),
        "Unclaimed non-planar owner",
    )
    results = ["duplicate_triangle", "split_owner", "remaining_analytic_owner"]
    if native is not None:
        import copy

        shifted = copy.deepcopy(native)
        p = shifted["loops"][0]["edges"][0]["start_mm"]
        p[0] += 0.1
        refused(
            lambda: match_native(shifted, *match_args),
            "Ambiguous native/source endpoint",
        )
        misplaced = copy.deepcopy(native)
        arc = next(
            edge
            for edge in misplaced["loops"][0]["edges"]
            if edge["kind"] == "circle" and abs(edge["signed_sweep_rad"]) > 1.0
        )
        arc["center_mm"] = (arc["center_mm"] + 0.2 * np.asarray(arc["axis"])).tolist()
        refused(
            lambda: match_native(misplaced, *match_args),
            "A matched source chord exceeds its circle tolerance",
        )
        results += ["shifted_native_endpoint", "misplaced_aggregated_circle"]
    if ends is not None:
        refused(
            lambda: rail_end_owners(ends[:1], old, nplanes), "Expected two rail ends"
        )
        refused(
            lambda: rail_end_owners([ends[0], ends[0]], old, nplanes),
            "Duplicate rail end",
        )
        wrong = [dict(ends[0], triangles=ends[1]["triangles"]), ends[1]]
        refused(
            lambda: rail_end_owners(wrong, old, nplanes),
            "Rail end does not cover its full measured owner",
        )
        results += [
            "missing_rail_end",
            "duplicate_rail_end",
            "exchanged_rail_end_triangles",
        ]
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dump", type=Path)
    parser.add_argument("assembly", type=Path)
    parser.add_argument("chains", type=Path)
    parser.add_argument("markings", type=Path)
    parser.add_argument(
        "--source-frontier",
        "--frontier",
        type=Path,
        help="Write measured directed edge uses locally",
    )
    parser.add_argument(
        "--native-frontier",
        type=Path,
        help="Cross-check the native OCCT boundary witness",
    )
    parser.add_argument("--check-negatives", action="store_true")
    parser.add_argument(
        "--rail-ends",
        type=Path,
        help="Include both certified owners from rail-ends.json",
    )
    trims = parser.add_mutually_exclusive_group()
    trims.add_argument(
        "--trim-neighbors",
        nargs=2,
        type=Path,
        metavar=("MANIFEST", "OUT"),
        help="Write all source edge uses of the trim triangles on the 1816-face shell",
    )
    trims.add_argument(
        "--include-trims",
        type=Path,
        metavar="MANIFEST",
        help="Count all qualified trim triangles and recompute the source frontier",
    )
    args = parser.parse_args()
    if (args.include_trims or args.trim_neighbors) and not args.rail_ends:
        parser.error("Trim measurements require the 1816-face shell (--rail-ends)")
    ends = None
    if args.rail_ends:
        require(args.rail_ends.stat().st_size <= 32 * 1024, "Oversize rail end witness")
        ends = json.loads(args.rail_ends.read_text())
        require(isinstance(ends, list), "Invalid rail end witness")
    old, nplanes, analytic, hist, groups, names, counts, summary = attribution(
        args.dump, args.assembly, args.chains, args.markings, ends
    )
    vertices, triangles, pairs = read_incidence(args.dump, old, nplanes, analytic)
    manifest = args.include_trims or (
        args.trim_neighbors[0] if args.trim_neighbors else None
    )
    extra_refusals = []
    if manifest:
        rows = trim_rows(manifest)
        trim_args = (old, nplanes, hist, groups, vertices, triangles)
        ids = trim_ids(rows, *trim_args)
        if args.check_negatives:
            extra_refusals = trim_refusals(rows, *trim_args)
        if args.trim_neighbors:
            summary["trim_neighbor_uses"] = trim_neighbors(
                args.trim_neighbors[1], ids, old, groups, vertices, triangles, pairs
            )
        else:
            for i in ids:
                name = f"trim-{old[i]}"
                groups[i] = len(names)
                names.append(name)
                counts[name] = 1
            summary = checked_partition(old, nplanes, hist, groups, names, counts)
    interfaces, xyz, a, b, frontier = interface_report(
        vertices, triangles, pairs, groups, old, names, args.source_frontier
    )
    native = (
        json.loads(args.native_frontier.read_text()) if args.native_frontier else None
    )
    match_args = (xyz, a, b, frontier)
    if native is not None:
        interfaces["native_correspondence"] = match_native(native, *match_args)
    if args.check_negatives:
        interfaces["refused_controls"] = (
            refusal_controls(
                old, nplanes, hist, groups, names, counts, native, match_args, ends
            )
            + extra_refusals
        )
    print(json.dumps({**summary, **interfaces}, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
