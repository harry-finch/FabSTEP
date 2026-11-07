"""Check M4 STEP outputs against the independent analytical fixture manifest.

Run with the Python bundled with FreeCAD, and pass the directory of STEP files.
"""
import json
import math
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
CASES = json.loads((HERE.parent / "fixtures/m4_manifest.json").read_text())["cases"]


def close(a, b):
    return math.isclose(a, b, rel_tol=1e-8, abs_tol=1e-7)


def validate(path, references, boxes):
    expected = sorted(zip(boxes, references), key=lambda item: item[0])
    bounds = [list(map(fn, zip(*(box[i] for box in boxes)))) for i, fn in enumerate((min, max))]
    cylinders = [c for ref in references for c in ref["cylinders"]]
    cones = [c for ref in references for c in ref["cones"]]
    args = {
        "--solids": [len(references)],
        "--faces": [sum(ref["faces"] for ref in references)],
        "--planes": [sum(ref["planes"] for ref in references)],
        "--cylinders": [len(cylinders)],
        "--cones": [len(cones)],
        "--volume": [math.fsum(ref["volume_mm3"] for ref in references)],
        "--bounds": sum(bounds, []),
        "--axis": references[0].get("axis", []),
        "--radii": [c["radius_mm"] for c in cylinders],
        "--heights": [c["height_mm"] for c in cylinders],
        "--cone-end-radii": [r for c in cones for r in c["end_radii_mm"]],
        "--cone-heights": [c["height_mm"] for c in cones],
        "--cone-angles-deg": [c["semi_angle_deg"] for c in cones],
    }
    cmd = [sys.executable, str(HERE / "validate-freecad.py"), str(path)]
    for option, values in args.items():
        if values:
            cmd += [option, *map(str, values)]
    process = subprocess.run(cmd, stdout=subprocess.PIPE, text=True)
    if not process.stdout:
        raise SystemExit(f"Impossible de mesurer {path}. Code {process.returncode}.")
    report = json.loads(process.stdout)
    actual = sorted(report["bodies"], key=lambda body: body["bounds_mm"])
    report["passed"] = report["passed"] and len(actual) == len(expected) and all(
        body["valid"]
        and body["closed"]
        and body["faces"] == ref["faces"]
        and body["planes"] == ref["planes"]
        and body["cylinders"] == len(ref["cylinders"])
        and body["cones"] == len(ref["cones"])
        and close(body["volume_mm3"], ref["volume_mm3"])
        and all(close(a, b) for a, b in zip(sum(body["bounds_mm"], []), sum(box, [])))
        for body, (box, ref) in zip(actual, expected)
    )
    return report


if len(sys.argv) != 2:
    raise SystemExit("Usage : python scripts/validate-m4-freecad.py <répertoire STEP>")
directory = Path(sys.argv[1])
reports = {}
for case, entry in CASES.items():
    if "expected_refusal" in entry:
        continue
    references = [CASES[name] for name in entry["bodies"]] if "bodies" in entry else [entry]
    boxes = entry["body_bounds_mm"] if "bodies" in entry else [entry["bounds_mm"]]
    reports[case] = validate(directory / (case + ".step"), references, boxes)
passed = all(report["passed"] for report in reports.values())
print(json.dumps({"cases": reports, "passed": passed}, indent=2))
raise SystemExit(0 if passed else 1)
