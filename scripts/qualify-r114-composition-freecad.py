"""Check local patch ownership on small, independent FreeCAD/STL references.

PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python scripts/qualify-r114-composition-freecad.py <output-dir>

This is an oracle for composition of already certified candidates. It does not
discover surfaces or produce FabSTEP BReps. All coordinates are millimetres.
"""

import argparse
import json
import math
from collections import Counter, defaultdict
from pathlib import Path

import FreeCAD as App
import MeshPart
import Part

from r113_mesh import read_stl

V = App.Vector


def reference(conflict):
    body = Part.makeBox(24, 20, 18, V(-12, -10, -9))
    if not conflict:
        (corner,) = [
            e for e in body.Edges if (e.CenterOfMass - V(0, -10, 9)).Length < 1e-8
        ]
        body = body.makeFillet(1.5, [corner])
    cutters = [(6, 26, V(0, 0, 0), V(1, 0, 0))]
    stations = (-5, 5) if conflict else (0,)
    cutters.extend((3, 20, V(x, 0, 0), V(0, 0, 1)) for x in stations)
    for radius, length, center, axis in cutters:
        bore = Part.makeCylinder(radius, length, center - (length / 2) * axis, axis)
        body = body.cut(bore)
    return body.removeSplitter()


def label(face):
    if isinstance(face.Surface, Part.Plane):
        return "plane"
    if isinstance(face.Surface, Part.Cylinder):
        surface = face.Surface
        if abs(surface.Radius - 1.5) < 1e-8:
            return "fillet"
        if abs(surface.Radius - 6) < 1e-8 and abs(surface.Axis.x) > 1 - 1e-10:
            return "X"
        if abs(surface.Radius - 3) < 1e-8 and abs(surface.Axis.z) > 1 - 1e-10:
            return "Z2" if surface.Center.x > 1 else "Z"
    raise ValueError("unexpected surface in the transferred reference")


def source_groups(mesh, shape, path):
    vertices, triangles = read_stl(path)
    facets = mesh.Facets
    groups = defaultdict(set)
    assigned = set()
    if mesh.countSegments() != len(shape.Faces) or mesh.CountFacets != len(triangles):
        raise ValueError("mesher/source size mismatch")
    for face_index, face in enumerate(shape.Faces):
        owner = label(face)
        ids = set(mesh.getSegment(face_index))
        if not ids or assigned & ids:
            raise ValueError("missing or duplicate reference owner")
        for tid in ids:
            # The mesher's segment IDs must match the actual STL facets and winding.
            # Each float32 corner must still be on its claimed analytic support.
            points = tuple(vertices[i] for i in triangles[tid])
            if tuple(map(tuple, facets[tid].Points)) != points:
                raise ValueError("STL facet order differs")
            for p in points:
                if isinstance(face.Surface, Part.Cylinder):
                    distance = V(*p).distanceToLine(
                        face.Surface.Center, face.Surface.Axis
                    )
                    error = abs(distance - face.Surface.Radius)
                else:
                    error = abs((V(*p) - face.Surface.Position).dot(face.Surface.Axis))
                if error > 5e-6:
                    raise ValueError(
                        "reference owner inconsistent with f32 coordinates"
                    )
        groups[owner].update(ids)
        assigned.update(ids)
    if assigned != set(range(len(triangles))):
        raise ValueError("incomplete reference ownership")
    return triangles, groups


def boundary(triangles, ids):
    """Oriented external edges; two opposite uses of an interior edge cancel."""
    edges = set()
    for tid in ids:
        a, b, c = triangles[tid]
        for left, right in ((a, b), (b, c), (c, a)):
            if (right, left) in edges:
                edges.remove((right, left))
            elif (left, right) in edges:
                raise ValueError("repeated directed edge")
            else:
                edges.add((left, right))
    return edges


def validate_claims(triangles, claims):
    assigned = set()
    for ids, proposed_boundary in claims:
        if not ids or assigned & ids:
            raise ValueError("overlapping triangle ownership")
        if any(i < 0 or i >= len(triangles) for i in ids):
            raise ValueError("unknown source triangle")
        derived = boundary(triangles, ids)
        if proposed_boundary != derived:
            raise ValueError("proposed boundary differs from oriented source boundary")
        incoming = Counter(b for a, b in derived)
        outgoing = Counter(a for a, b in derived)
        if (
            not derived
            or incoming != outgoing
            or any(d != 1 for d in incoming.values())
        ):
            raise ValueError("open or branched patch boundary")
        assigned.update(ids)
    # At a shared edge either two accepted patches meet, or one meets a
    # retained source face. All such edges have exactly two opposite uses.
    remaining = set(range(len(triangles))) - assigned
    uses = defaultdict(list)
    for directed in [edges for _, edges in claims] + [boundary(triangles, remaining)]:
        for a, b in directed:
            uses[min(a, b), max(a, b)].append((a, b))
    if any(len(pair) != 2 or pair[0] != pair[1][::-1] for pair in uses.values()):
        raise ValueError("edge not matched by an opposite use")
    return {
        "claimed_triangles": len(assigned),
        "retained_triangles": len(remaining),
        "boundary_edges": len(uses),
    }


def evaluate(mesh, shape, path, conflict):
    triangles, groups = source_groups(mesh, shape, path)
    first = groups["X"] | groups["Z"]
    second = groups["X"] | groups["Z2"] if conflict else groups["fillet"]
    claims = [(ids, boundary(triangles, ids)) for ids in [first, second]]
    if conflict:
        # Both cross-drill candidates would own the X bore's whole wall. Trying
        # either one first must never allow the second to consume its triangles.
        overlap = first & second
        if not overlap or overlap != groups["X"]:
            raise ValueError("negative witness lacks the intended common wall")
        for ordered in [claims, claims[::-1]]:
            try:
                validate_claims(triangles, ordered)
            except ValueError as error:
                if str(error) != "overlapping triangle ownership":
                    raise
            else:
                raise ValueError("overlapping candidates accepted")
        return {
            "triangles": len(triangles),
            "common_wall_triangles": len(overlap),
            "both_orders": "refused",
        }
    result = validate_claims(triangles, claims)
    if result != validate_claims(triangles, claims[::-1]) or result[
        "retained_triangles"
    ] != len(groups["plane"]):
        raise ValueError("positive witness has order-dependent/incomplete ownership")
    # Even with disjoint triangle sets, a missing or reversed edge must refuse.
    other_ids, edges = claims[1]
    a, b = min(edges)
    for wrong in [edges - {(a, b)}, edges - {(a, b)} | {(b, a)}]:
        try:
            validate_claims(triangles, [claims[0], (other_ids, wrong)])
        except ValueError as error:
            if str(error) != "proposed boundary differs from oriented source boundary":
                raise
        else:
            raise ValueError("incorrect border accepted")
    return {
        "triangles": len(triangles),
        **result,
        "both_orders": "accepted",
        "missing_or_reversed_edge": "refused",
    }


def run(output):
    output.mkdir(parents=True, exist_ok=True)
    Part.setStaticValue("write.surfacecurve.mode", 1)
    for name, conflict in [("disjoint", False), ("common-bore", True)]:
        step = output / (name + ".step")
        ref = reference(conflict)
        ref.check(True)
        ref.exportStep(str(step))
        shape = Part.read(str(step))
        if (
            len(shape.Solids) != 1
            or not shape.isClosed()
            or not shape.isValid()
            or not math.isclose(ref.Volume, shape.Volume, rel_tol=1e-8)
        ):
            raise ValueError("invalid transferred reference")
        shape.check(True)
        results = {}
        for suffix, angle in [("coarse", 0.2), ("fine", 0.1)]:
            mesh = MeshPart.meshFromShape(
                Shape=shape,
                LinearDeflection=0.025,
                AngularDeflection=angle,
                Relative=False,
                Segments=True,
            )
            path = output / f"{name}-{suffix}.stl"
            mesh.write(str(path))
            results[suffix] = evaluate(mesh, shape, path, conflict)
        (output / (name + ".json")).write_text(json.dumps(results, indent=2) + "\n")
        print(name, json.dumps(results))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    run(parser.parse_args().output)
