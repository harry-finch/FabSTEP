"""Count the whole Flansch document after measured faces replace complete automatic owners.

The proof concerns ownership. It does not join boundaries or construct a solid.
"""
import argparse
import json
from pathlib import Path

import numpy as np


def require(test, message):
    if not test:
        raise ValueError(message)


CONE_IDS = (3, 5, 7, 9, 11)
LABELS = ([f"torus-{i}" for i in range(11)] + [f"lower-tube-{i}" for i in range(5)]
          + [f"upper-tube-{i}" for i in range(5)] + [f"short-flank-{i}" for i in range(4)]
          + ["side-plane-0", "side-plane-2", "side-plane-decorated", "base-plane"]
          + [f"cone-{i}" for i in CONE_IDS])


def automatic_owners(directory):
    profile = json.loads((directory/"residual-planar.json").read_text())
    n = profile["source_triangles"]
    file = directory/"owners.bin"
    require(0 < n <= 600000 and file.stat().st_size == 4*n, "Incomplete automatic owner dump")
    previous = np.fromfile(file, dtype="<u4")
    planar = profile["planar_owners"]
    analytic = sorted(profile["analytic_owners"])
    total = planar + len(analytic)
    require(0 < total <= len(previous) and previous.max() < total, "Out-of-range automatic owner")
    hist = np.bincount(previous, minlength=total)
    require((hist > 0).all() and hist[:planar].sum() == profile["planar_triangles"]
            and hist[planar:].tolist() == [profile["analytic_owners"][k] for k in analytic],
            "Recognition counts differ from automatic owners")
    sizes, counts = np.unique(hist[:planar], return_counts=True)
    require(np.array_equal(np.column_stack([sizes, counts]), profile["triangles_per_owner_histogram"]),
            "Plane-owner histogram mismatch")
    return previous, planar, analytic, hist


def load_groups(stream, n, labels):
    groups = np.full(n, -1, dtype=np.int16)
    indices = {k: i for i, k in enumerate(labels)}
    for line in stream:
        triangle, label = line.rstrip("\n").split("\t")
        i = int(triangle)
        require(0 <= i < n and label in indices, "Invalid patch row")
        require(groups[i] == -1, "Repeated triangle assignment")
        groups[i] = indices[label]
    return groups


def audit(previous, planar, analytic, hist, groups, counts, labels):
    total = len(hist)
    # Minimum and maximum range over EVERY triangle of each old owner,
    # including the sentinel (-1) for triangles outside the measured groups.
    # Equality rules out both partial replacements and splits across groups.
    lo = np.full(total, len(labels), dtype=groups.dtype)
    hi = np.full(total, -1, dtype=groups.dtype)
    np.minimum.at(lo, previous, groups)
    np.maximum.at(hi, previous, groups)
    require((lo == hi).all(), "Partially replaced or split automatic owner")
    used = lo >= 0
    expected = np.array([counts[k] for k in labels])
    actual = np.bincount(groups[groups >= 0], minlength=len(labels))
    require((actual == expected).all(), "Patch count differs from qualified source")
    merged = np.bincount(lo[used], minlength=len(labels))
    require((merged > 0).all(), "Missing measured group")
    preserved_cones = {f"Cone {{ index: {i} }}": labels.index(f"cone-{i}") for i in CONE_IDS}
    for i, name in enumerate(analytic, start=planar):
        if name in preserved_cones:
            slot = preserved_cones[name]
            require(lo[i] == slot and merged[slot] == 1, "Cone owner does not match its measured ring")
        else:
            require(not used[i], "Unrelated analytic owner was replaced")
    require(set(preserved_cones).issubset(analytic), "Missing original cone")
    # Assign disjoint integer namespaces for unchanged owners and local faces,
    # then independently count all final ids rather than just subtracting.
    final = np.where(groups >= 0, groups.astype(np.int64) + total, previous)
    ids, coverage = np.unique(final, return_counts=True)
    unchanged = np.count_nonzero(~used)
    require(np.array_equal(ids, np.r_[np.flatnonzero(~used), total + np.arange(len(labels))])
            and np.array_equal(coverage, np.r_[hist[~used], expected])
            and coverage.sum() == len(previous), "Incomplete document partition")
    return dict(source_triangles=len(previous), measured_triangles=int(actual.sum()),
                unchanged_triangles=int(hist[~used].sum()), original_faces=total,
                measured_original_faces=int(used.sum()), unchanged_faces=int(unchanged),
                measured_faces=len(labels), hypothetical_faces=len(ids),
                over_result_limit_by=max(0, len(ids)-100000),
                groups={label: dict(triangles=int(n), original_faces=int(old))
                        for label, n, old in zip(labels, actual, merged, strict=True)})


def refusals(previous, planar, analytic, hist, groups, counts, labels):
    def rejected(thunk, reason):
        try:
            thunk()
        except ValueError as problem:
            if str(problem) == reason:
                return
            raise
        raise AssertionError(f"Accepted damaged assignment: {reason}")

    duplicate = [f"0\t{labels[0]}\n"] * 2
    rejected(lambda: load_groups(duplicate, len(previous), labels), "Repeated triangle assignment")
    selected = groups >= 0
    many = np.flatnonzero(selected & (hist[previous] > 1))[0]
    # Mutating one triangle either leaves an old owner partly present or
    # spreads it across two candidate surfaces.
    for value in (-1, (groups[many]+1) % len(labels)):
        wrong = groups.copy()
        wrong[many] = value
        rejected(lambda: audit(previous, planar, analytic, hist, wrong, counts, labels),
                 "Partially replaced or split automatic owner")
    single = np.flatnonzero(selected & (hist[previous] == 1))[0]
    wrong = groups.copy()
    wrong[single] = -1
    rejected(lambda: audit(previous, planar, analytic, hist, wrong, counts, labels),
             "Patch count differs from qualified source")
    wrong = groups.copy()
    a, b = (labels.index(f"cone-{i}") for i in CONE_IDS[:2])
    wrong[groups == a], wrong[groups == b] = b, a
    rejected(lambda: audit(previous, planar, analytic, hist, wrong, counts, labels),
             "Cone owner does not match its measured ring")
    return ["duplicate", "partial_old_owner", "split_old_owner", "missing_singleton", "swapped_cones"]


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("automatic", type=Path, help="R-080 directory with FABSTEP_DUMP_OWNERS=1")
    parser.add_argument("patches", type=Path, help="Analysis triangle and measured face, tab-separated")
    parser.add_argument("counts", type=Path, help="Expected triangle counts, keyed by measured face")
    parser.add_argument("--check-negatives", action="store_true")
    parser.add_argument("--long-flanks", action="store_true")
    args = parser.parse_args()
    labels = LABELS + ([f"long-flank-{i}" for i in (4,5,6)] if args.long_flanks else [])
    original = automatic_owners(args.automatic)
    counts = json.loads(args.counts.read_text())
    require(set(counts) == set(labels) and all(type(c) is int and c > 0 for c in counts.values()),
            "Expected positive counts for all measured faces")
    with args.patches.open() as stream:
        groups = load_groups(stream, len(original[0]), labels)
    result = audit(*original, groups, counts, labels)
    if args.check_negatives:
        result["refused_controls"] = refusals(*original, groups, counts, labels)
    print(json.dumps(result, indent=2, allow_nan=False))
