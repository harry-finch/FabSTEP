"""Verify full-sphere STEP outputs and the cospherical clipping control."""
import argparse
import json
import math
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
CASES = json.loads((HERE.parent / "fixtures/m5c_manifest.json").read_text())["cases"]


def close(a, b):
    return math.isclose(a, b, rel_tol=1e-8, abs_tol=2e-7)


def validate(path, refs):
    args = [sys.executable, str(HERE / "validate-freecad.py"), str(path)]
    totals = {
        "--solids": len(refs),
        "--faces": sum(ref["faces"] for ref in refs),
        "--planes": sum(ref["planes"] for ref in refs),
        "--cylinders": 0, "--cones": 0, "--tori": 0,
        "--spheres": sum(len(ref["spheres"]) for ref in refs),
        "--volume": math.fsum(ref["volume_mm3"] for ref in refs),
    }
    for option, value in totals.items():
        args += [option, str(value)]
    process = subprocess.run(args, stdout=subprocess.PIPE, text=True)
    if not process.stdout:
        raise SystemExit(f"Impossible de mesurer {path}. Code {process.returncode}.")
    report = json.loads(process.stdout)
    actual = sorted(report["bodies"], key=lambda body: body["bounds_mm"])
    expected = sorted(refs, key=lambda ref: ref["bounds_mm"])
    report["passed"] = report["passed"] and len(actual) == len(expected) and all(
        body["valid"] and body["closed"]
        and body["faces"] == ref["faces"] and body["planes"] == ref["planes"]
        and body["spheres"] == len(ref["spheres"])
        and close(body["volume_mm3"], ref["volume_mm3"])
        and all(close(a, b) for a, b in zip(sum(body["bounds_mm"], []), sum(ref["bounds_mm"], [])))
        for body, ref in zip(actual, expected)
    )

    def matches(surface, nominal):
        latitudes = surface["latitudes_deg"]
        if surface["axis"][2] < 0.0:
            latitudes = [-v for v in latitudes[::-1]]
        return (
            not surface["inner"]
            and close(surface["radius_mm"], nominal["radius_mm"])
            and all(close(a, b) for a, b in zip(surface["center_mm"], nominal["center_mm"]))
            and all(close(a, b) for a, b in zip(latitudes, nominal["latitudes_deg"]))
        )

    measured = sorted(report["spheres"], key=lambda s: s["center_mm"])
    nominal = sorted((sphere for ref in refs for sphere in ref["spheres"]),
                     key=lambda sphere: sphere["center_mm"])
    report["passed"] = report["passed"] and len(measured) == len(nominal) and all(
        matches(actual, expected) for actual, expected in zip(measured, nominal)
    )
    return report


def write_reference(path, refs):
    import FreeCAD
    import Part

    bodies = []
    for ref in refs:
        sphere, = ref["spheres"]
        bodies.append(Part.makeSphere(
            sphere["radius_mm"],
            FreeCAD.Vector(*sphere["center_mm"]),
            FreeCAD.Vector(0.0, 0.0, 1.0),
            *sphere["latitudes_deg"], 360.0,
        ))
    shape = bodies[0] if len(bodies) == 1 else Part.makeCompound(bodies)
    shape.exportStep(str(path))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--write-references", action="store_true",
                        help="Construire séparément les références avec FreeCAD avant mesure")
    args = parser.parse_args()
    if args.write_references:
        args.directory.mkdir(parents=True, exist_ok=True)
    reports = {}
    for name, entry in CASES.items():
        if "expected_refusal" in entry:
            continue
        refs = entry.get("bodies", [entry])
        path = args.directory / (name + ".step")
        if args.write_references:
            write_reference(path, refs)
        reports[name] = validate(path, refs)
    passed = all(report["passed"] for report in reports.values())
    print(json.dumps({"cases": reports, "passed": passed}, indent=2))
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
