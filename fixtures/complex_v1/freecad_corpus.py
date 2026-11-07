"""Construct and measure the mechanical acceptance corpus using FreeCAD Part."""

import argparse
from collections import Counter
import json
import math
from pathlib import Path

import FreeCAD as App
import Part

V = App.Vector
HERE = Path(__file__).resolve().parent
MANIFEST = json.loads((HERE / "manifest.json").read_text())
DEFAULT_ROOT = HERE.parents[1] / "artifacts" / "complex-v1"


def manifold():
    block = Part.makeBox(48, 36, 18)
    through = Part.makeCylinder(3, 20, V(10, 10, -1))
    blind_x = Part.makeCylinder(2, 9, V(49, 13, 8), V(-1, 0, 0))
    blind_y = Part.makeCylinder(2.5, 10, V(28, -1, 8), V(0, 1, 0))
    return block.cut(through).cut(blind_x).cut(blind_y)


def double_chamfer():
    profile = [(5, 0), (12, 0), (15, 3), (15, 13), (11, 17), (5, 17)]
    polygon = Part.makePolygon([V(r, 0, z) for r, z in profile + profile[:1]])
    return Part.Face(polygon).revolve(V(), V(0, 0, 1), 360)


def mixed_housing():
    block = Part.makeBox(50, 40, 16)
    boss = Part.makeCylinder(8, 8, V(34, 26, 16))
    top_edge = [edge for edge in boss.Edges if edge.BoundBox.ZMin > 23.99]
    assert len(top_edge) == 1
    rounded_boss = boss.makeFillet(2, top_edge)
    through = Part.makeCylinder(3, 18, V(14, 13, -1))
    countersink = Part.makeCone(3, 6, 3, V(14, 13, 14))
    blind = Part.makeCylinder(2, 9, V(51, 10, 8), V(-1, 0, 0))
    cup = Part.makeSphere(5, V(14, 28, 18))
    return block.fuse(rounded_boss).cut(through.fuse(countersink)).cut(blind).cut(cup)


def rounded_enclosure():
    block = Part.makeBox(28, 20, 14)
    return block.makeFillet(2, block.Edges)


def elliptic_bore():
    ellipse = Part.Ellipse(V(14, 13, -1), 5, 4)
    bore = Part.Face(Part.Wire([ellipse.toShape()])).extrude(V(0, 0, 12))
    return Part.makeBox(28, 26, 10).cut(bore)


CONSTRUCTORS = {
    "01-manifold": manifold,
    "02-double-chamfer": double_chamfer,
    "03-mixed-housing": mixed_housing,
    "04-rounded-enclosure": rounded_enclosure,
    "05-elliptic-bore": elliptic_bore,
}


def write_stl(path, points, faces):
    with path.open("w") as output:
        output.write(f"solid {path.stem}\n")
        for indices in faces:
            output.write(" facet normal 0 0 0\n  outer loop\n")
            for i in indices:
                point = points[i]
                output.write(f"   vertex {point.x:.17g} {point.y:.17g} {point.z:.17g}\n")
            output.write("  endloop\n endfacet\n")
        output.write(f"endsolid {path.stem}\n")


def tangent_edges(shape, expected_pairs):
    counts = Counter()
    worst_angle = 0.0
    for edge in shape.Edges:
        faces = shape.ancestorsOfType(edge, Part.Face)
        if len(faces) != 2:
            continue
        key = "/".join(sorted(type(face.Surface).__name__ for face in faces))
        if key not in expected_pairs:
            continue
        counts[key] += 1
        for fraction in (0.25, 0.5, 0.75):
            parameter = edge.FirstParameter + fraction * (edge.LastParameter - edge.FirstParameter)
            point = edge.valueAt(parameter)
            left, right = [face.normalAt(*face.Surface.parameter(point)) for face in faces]
            dot = max(-1.0, min(1.0, left.dot(right)))
            worst_angle = max(worst_angle, math.degrees(math.acos(dot)))
    return dict(counts), worst_angle


def inspect(shape, case, *, reference):
    bounds = shape.BoundBox
    report = {
        "id": case["id"],
        "solids": len(shape.Solids),
        "valid": shape.isValid(),
        "closed": shape.isClosed(),
        "volume_mm3": shape.Volume,
        "bounds_mm": [bounds.XMin, bounds.YMin, bounds.ZMin, bounds.XMax, bounds.YMax, bounds.ZMax],
    }
    surfaces = [face.Surface for face in shape.Faces]
    report["surfaces"] = dict(Counter(type(s).__name__ for s in surfaces))
    report["cylinder_radii_mm"] = sorted(
        surface.Radius for surface in surfaces if isinstance(surface, Part.Cylinder)
    )
    report["cone_semi_angles_deg"] = sorted(
        abs(math.degrees(surface.SemiAngle)) for surface in surfaces if isinstance(surface, Part.Cone)
    )
    report["sphere_radii_mm"] = sorted(
        surface.Radius for surface in surfaces if isinstance(surface, Part.Sphere)
    )
    report["torus_radii_mm"] = sorted(
        [surface.MajorRadius, surface.MinorRadius] for surface in surfaces
        if isinstance(surface, Part.Toroid)
    )
    volume_tolerance = (case.get("reference_volume_tolerance_mm3", 1e-5) if reference
                        else max(0.1, case["volume_mm3"] * 1e-5))
    bounds_tolerance = 1e-5 if reference else 0.05
    parameter_tolerance = 1e-5 if reference else 0.01
    checks = {
        "topology": report["valid"] and report["closed"] and report["solids"] == 1,
        "volume": abs(report["volume_mm3"] - case["volume_mm3"]) <= volume_tolerance,
        "bounds": all(
            abs(a - b) <= bounds_tolerance for a, b in zip(report["bounds_mm"], case["bounds_mm"])
        ),
        "probes": all(
            shape.isInside(V(*probe["point_mm"]), 1e-6, False) == probe["inside"]
            for probe in case["probes"]
        ),
    }
    if "surfaces" in case:
        checks["surface_counts"] = report["surfaces"] == case["surfaces"]
    for name in ("cylinder_radii_mm", "cone_semi_angles_deg", "sphere_radii_mm", "torus_radii_mm"):
        if name in case:
            expected, actual = case[name], report[name]
            if name == "torus_radii_mm":
                expected = [radius for pair in expected for radius in pair]
                actual = [radius for pair in actual for radius in pair]
            checks[name] = len(expected) == len(actual) and all(
                abs(a - b) <= parameter_tolerance for a, b in zip(expected, actual)
            )
    if "tangent_pairs" in case:
        counts, worst_angle = tangent_edges(shape, case["tangent_pairs"])
        report["tangent_pairs"] = counts
        report["max_sampled_tangent_angle_deg"] = worst_angle
        checks["tangency"] = counts == case["tangent_pairs"] and worst_angle <= 0.01
    report["checks"] = checks
    report["passed"] = all(checks.values())
    return report


def generate(directory):
    directory.mkdir(parents=True, exist_ok=True)
    reports = []
    for case in MANIFEST["cases"]:
        name = case["id"]
        if name not in CONSTRUCTORS:
            continue
        shape = CONSTRUCTORS[name]().removeSplitter()
        step = directory / f"{name}.step"
        shape.exportStep(str(step))
        report = inspect(Part.read(str(step)), case, reference=True)
        points, faces = shape.tessellate(MANIFEST["tessellation_deflection_mm"])
        assert 0 < len(faces) <= 100_000, (name, len(faces))
        write_stl(directory / f"{name}.stl", points, faces)
        report["triangles"] = len(faces)
        reports.append(report)
        if name == "01-manifold":
            write_stl(directory / "06-open-manifold.stl", points, faces[1:])
    return reports


def inspect_candidates(directory):
    reports = []
    for case in MANIFEST["cases"]:
        name = case["id"]
        step = directory / f"{name}.step"
        if case["expectation"] != "analytic":
            error = read_cli_report(directory / f"{name}.stderr")
            reports.append({
                "id": name,
                "passed": not step.exists() and error.get("code") == case["error_code"],
                "step_written": step.exists(),
                "error_code": error.get("code"),
            })
            continue
        if not step.is_file():
            error = read_cli_report(directory / f"{name}.stderr")
            reports.append({
                "id": name, "passed": False, "missing_step": True, "error_code": error.get("code")
            })
            continue
        report = inspect(Part.read(str(step)), case, reference=False)
        cli = read_cli_report(directory / f"{name}.json")
        report.update(audit_measurements(cli))
        report["checks"]["cli_settings_and_audit"] = report["audit_passed"]
        report["passed"] = all(report["checks"].values())
        reports.append(report)
    for case in MANIFEST["cases"]:
        if case["expectation"] == "analytic":
            continue
        name = case["id"] + "-planar"
        step = directory / f"{name}.step"
        if case["expectation"] == "reject-all":
            error = read_cli_report(directory / f"{name}.stderr")
            reports.append({
                "id": name,
                "passed": not step.exists() and error.get("code") == case["error_code"],
                "step_written": step.exists(),
                "error_code": error.get("code"),
            })
        elif step.is_file():
            faceted_case = {key: value for key, value in case.items() if key != "surfaces"}
            faceted_case["id"] = name
            report = inspect(Part.read(str(step)), faceted_case, reference=False)
            report["checks"]["faceted"] = set(report["surfaces"]) == {"Plane"}
            report["passed"] = all(report["checks"].values())
            reports.append(report)
        else:
            reports.append({"id": name, "passed": False, "missing_step": True})
    return reports


def read_cli_report(path):
    if path.is_file():
        try:
            return json.loads(path.read_text())
        except json.JSONDecodeError:
            pass
    return {}


def audit_measurements(cli):
    settings = MANIFEST["settings"]
    bodies = cli.get("bodies", [])
    report = {"audit_passed": False}
    if len(bodies) != 1:
        return report
    body = bodies[0]
    recognition = next((value for key, value in body.items() if key.endswith("recognition") and value), {})
    bound = recognition.get("report", {}).get("max_deviation_bound_mm")
    fidelity = body.get("fidelity") or {}
    report["composed_bound_mm"] = bound
    report["fidelity"] = fidelity
    used = cli.get("settings", {})
    values = [fidelity.get(key) for key in ("source_to_result_mm", "result_to_source_mm")]
    report["audit_passed"] = (
        used.get("reconstruction") == settings["mode"]
        and used.get("cylinder_fit_tolerance_mm") == settings["fit_mm"]
        and used.get("max_deviation_mm") == settings["max_deviation_mm"]
        and bound is not None and 0 <= bound <= settings["max_deviation_mm"]
        and all(value is not None and math.isfinite(value) and value >= 0 for value in values)
        and all(fidelity.get(key, 0) > 0 for key in ("source_samples", "result_samples"))
    )
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument("--candidates", type=Path, help="read FabSTEP outputs instead of generating references")
    args = parser.parse_args()
    if args.candidates is None:
        reports = generate(args.root / "reference")
        output = args.root / "reference" / "measurements.json"
    else:
        reports = inspect_candidates(args.candidates)
        output = args.candidates / "measurements.json"
    passed = all(report["passed"] for report in reports)
    report = {"freecad": ".".join(App.Version()[:3]), "passed": passed, "cases": reports}
    output.write_text(json.dumps(report, indent=2) + "\n")
    for report in reports:
        print(report["id"], "OK" if report["passed"] else report.get("checks", "FAIL"))
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
