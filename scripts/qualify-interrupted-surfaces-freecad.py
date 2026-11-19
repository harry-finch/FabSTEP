"""Independent millimetre CAD fixtures for trimmed surfaces; FreeCAD Python environment.

PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python scripts/qualify-interrupted-surfaces-freecad.py <output-dir>
Optional second argument: path to a freshly built FabSTEP CLI for all four Auto conversions.
"""

import argparse
import json
import math
import subprocess
from collections import Counter
from pathlib import Path
import FreeCAD as App
import MeshPart
import Part

V = App.Vector
Z = V(0, 0, 1)


def oblique_lip():
    blank = Part.makeCylinder(12, 8)
    (rim,) = [e for e in blank.Edges if abs(e.CenterOfMass.z - 8) < 1e-8]
    rounded = blank.makeFillet(2, [rim])
    cutter = Part.makeBox(50, 80, 80, V(0, -40, -40))
    cutter.rotate(V(0, 0, 0), V(0, 1, 0), 15)
    cutter.translate(V(-2, 0, 6.5))
    return rounded.cut(cutter).removeSplitter()


def cross_bored_shaft():
    blank = Part.makeCylinder(10, 16)
    bore = Part.makeCylinder(2.4, 40, V(-20, 3.2, 7.25), V(1, 0, 0))
    return blank.cut(bore).removeSplitter()


def verify(solid):
    assert len(solid.Solids) == 1 and solid.isValid() and solid.isClosed()
    solid.check(True)
    return {
        "solids": len(solid.Solids),
        "valid": solid.isValid(),
        "closed": solid.isClosed(),
        "volume_mm3": solid.Volume,
        "edges": len(solid.Edges),
        "surfaces": dict(Counter(f.Surface.TypeId for f in solid.Faces)),
        "bounds_mm": [
            getattr(solid.BoundBox, p)
            for p in ("XMin", "YMin", "ZMin", "XMax", "YMax", "ZMax")
        ],
    }


def target_edges(solid, model):
    found = []
    for edge in solid.Edges:
        faces = solid.ancestorsOfType(edge, Part.Face)
        if len(faces) != 2:
            continue
        types = sorted(f.Surface.TypeId for f in faces)
        if model == "oblique-lip":
            if types != ["Part::GeomPlane", "Part::GeomToroid"]:
                continue
            (torus,) = [
                f.Surface for f in faces if f.Surface.TypeId == "Part::GeomToroid"
            ]
            (plane,) = [
                f.Surface for f in faces if f.Surface.TypeId == "Part::GeomPlane"
            ]
            if not 1e-6 < abs(torus.Axis.dot(plane.Axis)) < 1 - 1e-6:
                continue
            analytic = torus
        else:
            if types != ["Part::GeomCylinder", "Part::GeomCylinder"]:
                continue
            if abs(faces[0].Surface.Axis.dot(faces[1].Surface.Axis)) > 1e-8:
                continue
            analytic = max((f.Surface for f in faces), key=lambda c: c.Radius)
        # Unwrap u,v relative to previous sample, only when the surface is periodic.
        points = edge.discretize(Number=129)
        uv = [analytic.parameter(p) for p in points]
        unwrapped = [list(uv[0])]
        for i, p in enumerate(uv[1:], 1):
            delta = [q - r for q, r in zip(p, uv[i - 1])]
            delta[0] = math.remainder(delta[0], math.tau)
            if model == "oblique-lip":
                delta[1] = math.remainder(delta[1], math.tau)
            unwrapped.append([a + d for a, d in zip(unwrapped[-1], delta)])
        spans = [
            max(p[j] for p in unwrapped) - min(p[j] for p in unwrapped)
            for j in range(2)
        ]
        assert min(spans) > 1e-3
        orients = [ee.Orientation for f in faces for ee in f.Edges if ee.isSame(edge)]
        assert sorted(orients) == ["Forward", "Reversed"], orients
        found.append(
            {
                "curve": edge.Curve.TypeId,
                "supports": types,
                "occurrences": orients,
                "length_mm": edge.Length,
                "support_uv_spans": spans,
            }
        )
    assert len(found) == 2, found
    return found


def run(d, binary=None):
    d.mkdir(parents=True, exist_ok=True)
    failed = []
    for name, shape in [
        ("oblique-lip", oblique_lip()),
        ("cross-bored-shaft", cross_bored_shaft()),
    ]:
        case = d / name
        case.mkdir(exist_ok=True)
        src = case / "reference.step"
        # Emit both supporting surface curves with the shared 3D edge.
        Part.setStaticValue("write.surfacecurve.mode", 1)
        shape.exportStep(str(src))
        reference = Part.read(str(src))
        s0, s1 = verify(shape), verify(reference)
        assert s0["surfaces"] == s1["surfaces"]
        assert s0["edges"] == s1["edges"]
        assert abs(s0["volume_mm3"] - s1["volume_mm3"]) < s0["volume_mm3"] * 1e-7
        seams = target_edges(reference, name)
        record = {
            "freecad": App.Version()[:3],
            "occ": Part.OCC_VERSION,
            "before_step": s0,
            "reference": s1,
            "target_shared_edges": seams,
            "meshes": {},
        }
        for label, lin, ang in [("coarse", 0.05, 0.26), ("fine", 0.025, 0.17)]:
            mesh = MeshPart.meshFromShape(
                Shape=reference,
                LinearDeflection=lin,
                AngularDeflection=ang,
                Relative=False,
            )
            assert mesh.isSolid()
            stl = case / (label + ".stl")
            mesh.write(str(stl))
            measurements = {
                "linear_mm": lin,
                "angular_rad": ang,
                "triangles": mesh.CountFacets,
            }
            record["meshes"][label] = measurements
            if binary:
                target = case / (label + "-auto.step")
                cmd = [
                    binary,
                    "convert",
                    str(stl),
                    "--mode",
                    "auto",
                    "--json",
                    "--force",
                    "-o",
                    str(target),
                ]
                proc = subprocess.run(cmd, capture_output=True)
                (case / (label + "-auto.stdout")).write_bytes(proc.stdout)
                (case / (label + "-auto.stderr")).write_bytes(proc.stderr)
                measurements["returncode"] = proc.returncode
                if proc.returncode == 0:
                    r = Part.read(str(target))
                    measurements["auto"] = verify(r)
                else:
                    measurements["error"] = proc.stderr.decode(errors="replace")[-1200:]
                    failed.append(f"{name}/{label}")
        (case / "evidence.json").write_text(json.dumps(record, indent=2) + "\n")
    if failed:
        raise SystemExit("Auto conversion failed for " + ", ".join(failed))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("cli", type=Path, nargs="?")
    args = parser.parse_args()
    run(args.output.resolve(), args.cli.resolve() if args.cli else None)
