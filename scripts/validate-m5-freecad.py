"""Verify M5 STEP outputs against the analytical fixture manifest with FreeCAD."""
import json
import math
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
CASES = json.loads((HERE.parent / "fixtures/m5_manifest.json").read_text())["cases"]


def close(a, b):
    return math.isclose(a, b, rel_tol=1e-8, abs_tol=1e-7)


def placed_bounds(ref, translation):
    return [[value + shift for value, shift in zip(row, translation)] for row in ref["bounds_mm"]]


def validate(path, refs, bounds):
    args = [sys.executable, str(HERE / "validate-freecad.py"), str(path)]
    totals = {
        "--solids": len(refs),
        "--faces": sum(ref["faces"] for ref in refs),
        "--planes": sum(ref["planes"] for ref in refs),
        "--cylinders": sum(ref["cylinders"] for ref in refs),
        "--cones": sum(ref["cones"] for ref in refs),
        "--spheres": sum(len(ref["spheres"]) for ref in refs),
        "--tori": sum(len(ref["tori"]) for ref in refs),
        "--toroidal-fillets": sum(ref.get("toroidal_fillets", 0) for ref in refs),
        "--volume": math.fsum(ref["volume_mm3"] for ref in refs),
    }
    for option, value in totals.items():
        args += [option, str(value)]
    sphere_radii = [s["radius_mm"] for ref in refs for s in ref["spheres"]]
    torus_radii = [r for ref in refs for t in ref["tori"] for r in (t["major_radius_mm"], t["minor_radius_mm"])]
    world = [fn(box[i][j] for box in bounds) for i, fn in enumerate((min, max)) for j in range(3)]
    for option, values in [
        ("--bounds", world), ("--axis", refs[0]["axis"]),
        ("--sphere-radii", sphere_radii), ("--torus-radii", torus_radii),
    ]:
        if values:
            args += [option, *map(str, values)]
    process = subprocess.run(args, stdout=subprocess.PIPE, text=True)
    if not process.stdout:
        raise SystemExit(f"Impossible de mesurer {path}. Code {process.returncode}.")
    report = json.loads(process.stdout)
    actual = sorted(report["bodies"], key=lambda body: body["bounds_mm"])
    expected = sorted(zip(bounds, refs), key=lambda item: item[0])
    report["passed"] = report["passed"] and len(actual) == len(expected) and all(
        body["valid"] and body["closed"]
        and all(body[key] == ref[key] for key in ("faces", "planes", "cylinders", "cones"))
        and body["spheres"] == len(ref["spheres"]) and body["tori"] == len(ref["tori"])
        and close(body["volume_mm3"], ref["volume_mm3"])
        and all(close(a, b) for a, b in zip(sum(body["bounds_mm"], []), sum(box, [])))
        for body, (box, ref) in zip(actual, expected)
    )
    for key, angle in (("spheres", "latitudes_deg"), ("tori", "meridian_deg")):
        wanted = sorted(s[angle] for ref in refs for s in ref[key])
        found = sorted(s[angle] for s in report[key])
        report["passed"] = report["passed"] and len(wanted) == len(found) and all(
            all(close(math.remainder(a - b, 360.), 0.) for a, b in zip(actual_range, expected_range))
            for actual_range, expected_range in zip(found, wanted)
        )
    return report


if len(sys.argv) != 2:
    raise SystemExit("Usage : python scripts/validate-m5-freecad.py <répertoire STEP>")
directory = Path(sys.argv[1])
reports = {}
for name, entry in CASES.items():
    if "expected_refusal" in entry:
        continue
    if "bodies" in entry:
        refs = [CASES[case] for case in entry["bodies"]]
        bounds = [placed_bounds(ref, shift) for ref, shift in zip(refs, entry["translations_mm"])]
    else:
        refs = [entry]
        bounds = [entry["bounds_mm"]]
    reports[name] = validate(directory / (name + ".step"), refs, bounds)
passed = all(report["passed"] for report in reports.values())
print(json.dumps({"cases": reports, "passed": passed}, indent=2))
raise SystemExit(0 if passed else 1)
