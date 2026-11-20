"""Fit an interrupted cylinder and torus from two independently meshed FreeCAD lips.

Run with the FreeCAD Python interpreter and its Resources/lib in PYTHONPATH.
The output directory contains the reference, both STL densities, the STEP built
from the fitted dimensions, and the JSON evidence. No private model is needed.
"""

import argparse
import importlib.util
import json
import math
import subprocess
from dataclasses import asdict
from pathlib import Path

import FreeCAD as App
import MeshPart
import Part

from r113_mesh import read_stl
from r113_shaft_fit import FIT, dot
from r125_lip_fit import fit

spec = importlib.util.spec_from_file_location(
    "interrupted", Path(__file__).with_name("qualify-interrupted-surfaces-freecad.py")
)
reference_checks = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reference_checks)
V = App.Vector


def shape(outer, height, fillet, offset, slope):
    blank = Part.makeCylinder(outer, height)
    (rim,) = [e for e in blank.Edges if abs(e.CenterOfMass.z - height) < 1e-8]
    rounded = blank.makeFillet(fillet, [rim])
    reach = 8 * max(outer, height)
    cutter = Part.makeBox(reach, 2 * reach, 2 * reach, V(0, -reach, -reach))
    cutter.rotate(V(), V(0, 1, 0), math.degrees(math.atan(slope)))
    cutter.translate(V(offset, 0, 0))
    return rounded.cut(cutter).removeSplitter()


def fitted_shape(lip):
    s = shape(lip.outer, lip.height, lip.minor, lip.cut_offset, lip.slope)
    # The angular origin is supplied only by the fitted cutting plane.
    rotation = App.Rotation(V(*lip.radial), V(), V(*lip.axis), "ZXY")
    s.Placement = App.Placement(V(*lip.origin), rotation)
    return s


def rejected(action):
    try:
        action()
    except ValueError as error:
        return str(error)
    raise AssertionError("counterexample accepted")


def run(dest, cli=None):
    dest.mkdir(parents=True, exist_ok=True)
    Part.setStaticValue("write.surfacecurve.mode", 1)
    reference_parameters = {
        "cap": (
            12,
            8,
            2,
            -2 - 6.5 * math.tan(math.radians(15)),
            math.tan(math.radians(15)),
        ),
        "wide": (15, 11, 2.8, -3, -0.17),
    }
    evidence = {}
    for name, dimensions in reference_parameters.items():
        drawing = shape(*dimensions)
        step = dest / f"{name}-reference.step"
        drawing.exportStep(str(step))
        original = Part.read(str(step))
        base = reference_checks.verify(original)
        for density, lin, ang in (("coarse", 0.05, 0.26), ("fine", 0.025, 0.17)):
            tag = f"{name}-{density}"
            mesh = MeshPart.meshFromShape(
                Shape=original,
                LinearDeflection=lin,
                AngularDeflection=ang,
                Relative=False,
                Segments=True,
            )
            stl = dest / (tag + ".stl")
            mesh.write(str(stl))
            points, triangles = read_stl(stl)
            lip, owners, charts = fit(points, triangles)
            reference_checks.compare_source_regions(
                stl, mesh, original, "oblique-lip", owners
            )
            step = dest / (tag + "-fitted.step")
            if cli is None:
                fitted_shape(lip).exportStep(str(step))
            else:
                command = [
                    str(cli.resolve()),
                    "convert",
                    str(stl),
                    "-o",
                    str(step),
                    "--mode",
                    "auto",
                    "--json",
                    "--force",
                ]
                report = json.loads(subprocess.check_output(command))
                body = report["bodies"][0]
                assert body["oblique_lip_recognition"] is not None
                assert (
                    max(
                        body["fidelity"][key]
                        for key in ("source_to_result_mm", "result_to_source_mm")
                    )
                    < 0.05
                )
            result = Part.read(str(step))
            measured = reference_checks.verify(result)
            assert (
                measured["surfaces"] == base["surfaces"]
                and measured["edges"] == base["edges"]
            )
            assert (
                abs(measured["volume_mm3"] - base["volume_mm3"])
                <= base["volume_mm3"] * 2e-6
            )
            assert (
                max(
                    abs(x - y) for x, y in zip(measured["bounds_mm"], base["bounds_mm"])
                )
                <= 2e-5
            )
            target = reference_checks.target_edges(result, "oblique-lip")
            assert {s["branch_sign"] for s in target} == {-1, 1}
            shared = set(v for t in owners["cut"] for v in triangles[t]) & set(
                v for t in owners["torus"] for v in triangles[t]
            )
            interior = [
                i
                for i in shared
                if abs(lip.local(points[i])[1]) > FIT
                and lip.shoulder + FIT < lip.local(points[i])[2] < lip.height - FIT
            ]
            assert interior
            moved = list(points)
            k = min(interior)
            moved[k] = tuple(a + 0.04 * b for a, b in zip(moved[k], lip.radial))
            # Remove the material strip bordering one of the two analytic
            # branches, then reverse a different seam facet in a second trial.
            positive = {i for i in interior if lip.local(points[i])[1] > 0}
            missing = {
                t for t in owners["torus"] if positive.intersection(triangles[t])
            }
            assert missing
            broken = [t for i, t in enumerate(triangles) if i not in missing]
            seam = min(t for t in owners["cut"] if k in triangles[t])
            bad_winding = list(triangles)
            bad_winding[seam] = bad_winding[seam][::-1]
            negatives = {
                "displaced_branch": rejected(lambda: fit(moved, triangles)),
                "missing_branch": rejected(lambda: fit(points, broken)),
                "reversed_seam": rejected(lambda: fit(points, bad_winding)),
            }
            # Manufacture no alternative dimensions: a rigid pose of the same
            # STL must give identical owners and correspondingly moved supports.
            placement = App.Placement(V(8, -17, 5), App.Rotation(V(2, 1, 3), 37))
            posed = [tuple(placement.multVec(V(*p))) for p in points]
            transformed, posed_owners, _ = fit(posed, triangles)
            assert posed_owners == owners
            assert (
                math.dist(transformed.origin, tuple(placement.multVec(V(*lip.origin))))
                < FIT
            )
            assert (
                dot(transformed.axis, tuple(placement.Rotation.multVec(V(*lip.axis))))
                > 1 - 1e-10
            )
            slots = [None] * len(triangles)
            for j, group in enumerate(("bottom", "top", "cut", "cylinder", "torus")):
                for i in owners[group]:
                    slots[i] = j
            digest = 0xCBF29CE484222325
            for slot in slots:
                digest = ((digest ^ slot) * 0x100000001B3) % 2**64
            evidence[tag] = {
                "source_record_owners_fnv64": f"{digest:016x}",
                "triangles": len(triangles),
                "supports": asdict(lip),
                "charts": charts,
                "owners": {name: len(ids) for name, ids in owners.items()},
                "negative": negatives,
                "step": measured,
                "branches": target,
            }
    # A real cut yields a tiny angular domain at the floor. The family must
    # refuse extrapolation in the ill-conditioned, almost tangent case.
    tangent = shape(12, 8, 2, -11.85, 0.27)
    mesh = MeshPart.meshFromShape(
        Shape=tangent, LinearDeflection=0.025, AngularDeflection=0.17, Relative=False
    )
    stl = dest / "near-tangent.stl"
    mesh.write(str(stl))
    evidence["near_tangent"] = rejected(lambda: fit(*read_stl(stl)))
    (dest / "qualification.json").write_text(json.dumps(evidence, indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--cli", type=Path, help="build and reimport actual FabSTEP Auto output"
    )
    args = parser.parse_args()
    run(args.output, args.cli)
