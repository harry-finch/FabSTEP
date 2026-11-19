"""Independent millimetre CAD fixtures for trimmed surfaces; FreeCAD Python environment.

PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python scripts/qualify-interrupted-surfaces-freecad.py <output-dir>
The OCCT SDK's DRAWEXE must be on PATH for the adaptive volume check.
Optional second argument: path to a freshly built FabSTEP CLI for all four Auto conversions.
"""

import argparse
import json
import math
import os
import re
import subprocess
import tempfile
from collections import Counter
from pathlib import Path
import FreeCAD as App
import MeshPart
import Part
import r113_lip
import r113_shaft
from r113_mesh import read_stl
from r113_shaft_fit import fit_shaft

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


def cross_bored_shaft(
    radius=10.0, height=16.0, bore_radius=2.4, offset=3.2, level=7.25, seam_degrees=0.0
):
    blank = Part.makeCylinder(radius, height)
    bore = Part.makeCylinder(
        bore_radius, 4 * radius, V(-2 * radius, offset, level), V(1, 0, 0)
    )
    if seam_degrees:
        bore.rotate(V(0, offset, level), V(1, 0, 0), seam_degrees)
    return blank.cut(bore).removeSplitter()


def verify(solid):
    assert len(solid.Solids) == 1 and solid.isValid() and solid.isClosed()
    solid.check(True)
    box = solid.optimalBoundingBox(False)
    return {
        "solids": len(solid.Solids),
        "valid": solid.isValid(),
        "closed": solid.isClosed(),
        "volume_mm3": solid.Volume,
        "edges": len(solid.Edges),
        "surfaces": dict(Counter(f.Surface.TypeId for f in solid.Faces)),
        "bounds_mm": [
            getattr(box, p) for p in ("XMin", "YMin", "ZMin", "XMax", "YMax", "ZMax")
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
        side = [p.y if model == "oblique-lip" else p.x for p in points]
        assert min(side) > 0 or max(side) < 0, "edge crosses the two branches"
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
        # Check the actual two traces transferred by STEP at common parameters.
        # This measurement is separate from the source chord interpolation bound.
        trace_errors = []
        for face in faces:
            pcurve, first, last = face.curveOnSurface(edge)
            assert (first, last) == edge.ParameterRange
            distances = []
            for i in range(129):
                parameter = first + (last - first) * (i / 128)
                q = pcurve.value(parameter)
                distances.append(
                    (face.Surface.value(q.x, q.y) - edge.valueAt(parameter)).Length
                )
            trace_errors.append(max(distances))
        assert max(trace_errors) <= edge.getTolerance(1, Part.Edge)
        found.append(
            {
                "curve": edge.Curve.TypeId,
                "supports": types,
                "occurrences": orients,
                "length_mm": edge.Length,
                "branch_sign": 1 if side[0] > 0 else -1,
                "support_uv_spans": spans,
                "sampled_step_trace_errors_mm": trace_errors,
            }
        )
    assert sorted(e["branch_sign"] for e in found) == [-1, 1], found
    return found


def reference_label(face, model):
    surface = face.Surface
    if isinstance(surface, Part.Plane):
        if abs(surface.Axis.z) < 0.99:
            return "cut"
        return "bottom" if abs(face.CenterOfMass.z) < 1e-7 else "top"
    if isinstance(surface, Part.Toroid):
        return "torus"
    if isinstance(surface, Part.Cylinder):
        if model == "oblique-lip":
            return "cylinder"
        return "shaft" if abs(surface.Axis.z) > 0.99 else "bore"
    raise ValueError("unexpected reference surface")


def compare_source_regions(stl, mesh, reference, name, regions):
    # The BRep mesher knows each facet's owner. Its in-memory indices are usable
    # as STL record IDs only after verifying the actual file order and winding.
    vertices, triangles = read_stl(stl)
    if len(triangles) != mesh.CountFacets or mesh.countSegments() != len(
        reference.Faces
    ):
        raise ValueError("reference mesh size or segmentation differs")
    for facet, triangle in zip(mesh.Facets, triangles):
        if tuple(map(tuple, facet.Points)) != tuple(vertices[i] for i in triangle):
            raise ValueError("STL record does not match reference facet")
    expected = {
        reference_label(face, name): sorted(mesh.getSegment(i))
        for i, face in enumerate(reference.Faces)
    }
    if len(expected) != len(reference.Faces) or expected != {
        k: sorted(v) for k, v in regions.items()
    }:
        raise ValueError("source triangle assigned to a different reference face")
    if sorted(i for group in expected.values() for i in group) != list(
        range(len(triangles))
    ):
        raise ValueError("incomplete reference partition")
    return len(triangles)


def adaptive_volume(solid):
    # FreeCAD Shape.Volume's default quadrature depends on seam placement
    # on these two trimmed cylinders. Integrate the actual reimported BRep.
    with tempfile.TemporaryDirectory() as directory:
        brep = Path(directory) / "shape.brep"
        solid.exportBrep(str(brep))
        result = subprocess.run(
            [
                "DRAWEXE",
                "-b",
                "-c",
                "pload MODELING; restore $::env(R113_BREP) s; puts [vpropsgk s 1e-11 1 1 0]",
            ],
            env={**os.environ, "R113_BREP": str(brep)},
            capture_output=True,
            check=True,
            text=True,
            timeout=30,
        )
    mass = re.search(r"Mass\s*:\s*([\d.eE+-]+)", result.stdout)
    error = re.search(
        r"Relative error of mass computation\s*:\s*([\d.eE+-]+)", result.stdout
    )
    if (
        not mass
        or not error
        or not 0 <= float(error[1]) <= 1e-9
        or not float(mass[1]) > 0
    ):
        raise ValueError("adaptive volume integration failed: " + result.stdout)
    return float(mass[1])


def reference_shaft_volume(radius, height, bore_radius, offset):
    # Integrate the material removed at y = offset + bore_radius * sin(t).
    # The bore stays strictly between the two caps.
    n = 512
    step = math.pi / n
    values = []
    for i in range(n + 1):
        t = i * step - math.pi / 2
        y = offset + bore_radius * math.sin(t)
        area = 4 * (bore_radius * math.cos(t)) ** 2 * math.sqrt(radius**2 - y**2)
        weight = 1 if i in (0, n) else 4 if i % 2 else 2
        values.append(weight * area)
    return math.pi * radius**2 * height - step * math.fsum(values) / 3


def check_fitted_supports(solid, fit):
    origin, z = V(*fit["shaft_origin"]), V(*fit["shaft_axis"])
    bore, x = V(*fit["bore_origin"]), V(*fit["bore_axis"])
    height, radius = fit["height_mm"], fit["bore_radius_mm"]
    cylinders = sorted(
        (f.Surface for f in solid.Faces if isinstance(f.Surface, Part.Cylinder)),
        key=lambda s: s.Radius,
    )
    assert len(cylinders) == 2
    for surface, center, axis, r in zip(
        cylinders, (bore, origin), (x, z), (radius, fit["shaft_radius_mm"])
    ):
        assert abs(surface.Radius - r) < 1e-7
        assert surface.Axis.cross(axis).Length < 1e-8
        assert (surface.Center - center).cross(axis).Length < 1e-7
    levels = []
    for f in solid.Faces:
        if isinstance(f.Surface, Part.Plane):
            assert f.Surface.Axis.cross(z).Length < 1e-8
            levels.append((f.CenterOfMass - origin).dot(z))
    assert (
        len(levels) == 2
        and max(abs(a - b) for a, b in zip(sorted(levels), (0.0, height))) < 1e-7
    )
    level = (bore - origin).dot(z)
    # Interior/exterior pairs on both sides of the bore, clear of the end caps.
    for direction, cap_distance in ((-1, level), (1, height - level)):
        assert cap_distance > radius
        assert not solid.isInside(bore + z * (direction * radius / 2), 1e-7, False)
        assert solid.isInside(
            bore + z * (direction * (cap_distance + radius) / 2), 1e-7, False
        )


def fit_and_transfer_shaft(stl, mesh, reference, design=(10.0, 16.0, 2.4, 3.2)):
    """Build a laboratory STEP from source measurements alone, then consult the oracle."""
    vertices, triangles = read_stl(stl)
    fit = fit_shaft(vertices, triangles)
    proof = r113_shaft.qualify_fit(vertices, triangles, fit)
    origin, axis = V(*fit["shaft_origin"]), V(*fit["shaft_axis"])
    bore, direction = V(*fit["bore_origin"]), V(*fit["bore_axis"])
    radius, height = fit["shaft_radius_mm"], fit["height_mm"]
    # Build in the measured orthogonal frame so both cylinders have stable
    # seams. Place the finished solid back at its measured world coordinates.
    y = axis.cross(direction)
    bore_local = V(-2 * radius, (bore - origin).dot(y), (bore - origin).dot(axis))
    blank = Part.makeCylinder(radius, height)
    tool = Part.makeCylinder(fit["bore_radius_mm"], 4 * radius, bore_local, V(1, 0, 0))
    shaft = blank.cut(tool).removeSplitter()
    shaft.Placement = App.Placement(origin, App.Rotation(direction, y, axis, "ZXY"))
    verify(shaft)
    step = stl.with_name(stl.stem + "-measured-lab.step")
    shaft.exportStep(str(step))
    reread = Part.read(str(step))
    result, expected = verify(reread), verify(reference)
    check_fitted_supports(reread, fit)
    assert (
        result["surfaces"]
        == expected["surfaces"]
        == {"Part::GeomCylinder": 2, "Part::GeomPlane": 2}
    )
    actual_volume, expected_volume = adaptive_volume(reread), adaptive_volume(reference)
    assert abs(expected_volume - reference_shaft_volume(*design)) < 1e-5
    assert abs(actual_volume - expected_volume) < 1e-6 * expected_volume
    assert (
        max(abs(a - b) for a, b in zip(result["bounds_mm"], expected["bounds_mm"]))
        < 2e-5
    )
    regions = dict(fit["source_regions"])
    if axis.z < 0:
        regions["top"], regions["bottom"] = regions["bottom"], regions["top"]
    matched = compare_source_regions(stl, mesh, reference, "cross-bored-shaft", regions)
    return {
        "measured": {k: v for k, v in fit.items() if k != "source_regions"},
        "rotated_f32": r113_shaft.qualify_fitted_pose(vertices, triangles, fit),
        "verified_source_records": matched,
        "fitted_surface_bounds_mm": proof["bidirectional_bounds_mm"],
        "adaptive_volume_mm3": actual_volume,
        "reference_adaptive_volume_mm3": expected_volume,
        "reimport": result,
        "target_shared_edges": target_edges(reread, "cross-bored-shaft"),
    }


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
                Segments=True,
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
            proof = (r113_lip if name == "oblique-lip" else r113_shaft).qualify(stl)
            proof["verified_source_records"] = compare_source_regions(
                stl, mesh, reference, name, proof["source_regions"]
            )
            measurements["source_proof"] = proof
            if name == "cross-bored-shaft":
                measurements["source_fit_step"] = fit_and_transfer_shaft(
                    stl, mesh, reference
                )
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
    # Different dimensions and an independently remeshed seam test the measured
    # proposal. Only the reference volume oracle sees the drawing dimensions.
    case = d / "shaft-variant"
    case.mkdir(exist_ok=True)
    design = (9.7, 15.5, 2.2, 2.8)
    step = case / "reference.step"
    # A half-turn moves the reference cutter's periodic seam. In FreeCAD
    # 1.1.1 the zero-angle reference for these dimensions fails after transfer.
    reference = cross_bored_shaft(*design, level=6.9, seam_degrees=180.0)
    verify(reference)
    reference.exportStep(str(step))
    reference = Part.read(str(step))
    verify(reference)
    mesh = MeshPart.meshFromShape(
        Shape=reference,
        LinearDeflection=0.025,
        AngularDeflection=0.17,
        Relative=False,
        Segments=True,
    )
    stl = case / "fine.stl"
    mesh.write(str(stl))
    proof = fit_and_transfer_shaft(stl, mesh, reference, design)
    (case / "evidence.json").write_text(json.dumps(proof, indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("cli", type=Path, nargs="?")
    args = parser.parse_args()
    run(args.output.resolve(), args.cli.resolve() if args.cli else None)
