"""Reimport full-torus STEP files and compare them with analytic references."""

import argparse
import json
import math
from pathlib import Path
import subprocess
import sys


HERE = Path(__file__).resolve().parent
CASES = json.loads((HERE.parent / "fixtures/m5d_manifest.json").read_text())["cases"]


def close(a, b, *, absolute=2e-6):
    return math.isclose(a, b, rel_tol=1e-8, abs_tol=absolute)


def validate(path, refs):
    count = str(len(refs))
    expected_volume = math.fsum(ref["volume_mm3"] for ref in refs)
    command = [
        sys.executable, str(HERE / "validate-freecad.py"), str(path),
        "--solids", count, "--faces", count, "--tori", count,
        "--planes", "0", "--cylinders", "0", "--cones", "0", "--spheres", "0",
        "--toroidal-fillets", "0", "--volume", str(expected_volume),
    ]
    process = subprocess.run(command, capture_output=True, text=True)
    if not process.stdout:
        raise SystemExit(f"Impossible de mesurer {path}. Code {process.returncode}.\n{process.stderr}")
    report = json.loads(process.stdout)
    actual_bodies = sorted(report["bodies"], key=lambda body: body["bounds_mm"])
    expected_bodies = sorted(refs, key=lambda ref: ref["bounds_mm"])
    actual_tori = sorted(report["tori"], key=lambda torus: torus["center_mm"])
    expected_tori = sorted((torus for ref in refs for torus in ref["tori"]), key=lambda torus: torus["center_mm"])
    checks = [
        body["valid"] and body["closed"] and body["faces"] == body["tori"] == 1
        and close(body["volume_mm3"], ref["volume_mm3"], absolute=2e-5)
        and all(close(a, b) for pair in zip(body["bounds_mm"], ref["bounds_mm"]) for a, b in zip(*pair))
        for body, ref in zip(actual_bodies, expected_bodies)
    ]
    for torus, ref in zip(actual_tori, expected_tori):
        axis_error = min(
            math.dist(torus["axis"], [sign * a for a in ref["axis"]])
            for sign in (-1, 1)
        )
        meridian_span = math.radians(abs(torus["meridian_deg"][1] - torus["meridian_deg"][0]))
        checks.append(
            axis_error <= 1e-6
            and math.dist(torus["center_mm"], ref["center_mm"]) <= 2e-6
            and close(torus["major_radius_mm"], ref["major_radius_mm"])
            and close(torus["minor_radius_mm"], ref["minor_radius_mm"])
            and close(torus["angular_span_rad"], ref["angular_span_rad"])
            and close(meridian_span, ref["meridian_span_rad"])
            and not torus["connections"] and not torus["tangent_fillet"]
        )
    report["passed"] = (
        process.returncode == 0 and report["passed"]
        and len(actual_bodies) == len(expected_bodies) and len(actual_tori) == len(expected_tori)
        and all(checks)
    )
    return report


def write_reference(path, refs):
    import FreeCAD
    import Part

    solids = []
    for ref in refs:
        torus, = ref["tori"]
        solids.append(Part.makeTorus(
            torus["major_radius_mm"],
            torus["minor_radius_mm"],
            FreeCAD.Vector(*torus["center_mm"]),
            FreeCAD.Vector(*torus["axis"]),
        ))
    shape = solids[0] if len(solids) == 1 else Part.makeCompound(solids)
    shape.exportStep(str(path))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--write-references", action="store_true")
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
