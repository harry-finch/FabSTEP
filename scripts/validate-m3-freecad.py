"""Contrôle indépendant des sorties M3, avec le Python de FreeCAD.

Usage : python scripts/validate-m3-freecad.py artifacts/m3
Le manifeste nominal provient de générateurs indépendants de FabSTEP.
"""
import json
import math
from pathlib import Path
import sys
import FreeCAD
import Part

manifest = json.loads((Path(__file__).resolve().parent.parent / "fixtures/m3_manifest.json").read_text())
directory = Path(sys.argv[1])
reports = []
def close(a, b):
    return math.isclose(a, b, rel_tol=1e-8, abs_tol=1e-7)
def bounds(shape):
    b = shape.BoundBox
    return [[b.XMin, b.YMin, b.ZMin], [b.XMax, b.YMax, b.ZMax]]
for case, expected in manifest["cases"].items():
    if not expected["valid"]:
        continue
    path = directory / (case + ".step")
    shape = Part.Shape()
    shape.read(str(path))
    actual = [{"bounds_mm": bounds(s), "volume_mm3": s.Volume, "valid": s.isValid(), "closed": s.isClosed(), "faces": len(s.Faces), "planes": sum(isinstance(f.Surface, Part.Plane) for f in s.Faces)} for s in shape.Solids]
    nominal = sorted(zip(expected["body_bounds_mm"], expected["volume_each_mm3"]), key=lambda p: p[0])
    actual.sort(key=lambda p: p["bounds_mm"])
    passed = shape.isValid() and shape.isClosed() and len(actual) == expected["solids"] and close(shape.Volume, sum(expected["volume_each_mm3"]))
    for a, (b, volume) in zip(actual, nominal):
        passed = passed and a["valid"] and a["closed"] and a["faces"] == 6 and a["planes"] == 6 and close(a["volume_mm3"], volume) and all(close(x, y) for r, s in zip(a["bounds_mm"], b) for x, y in zip(r, s))
    reports.append({"case": case, "file": str(path), "passed": passed, "solids": len(actual), "volume_mm3": shape.Volume, "bodies": actual})
report = {"freecad": ".".join(FreeCAD.Version()[:3]), "cases": reports, "passed": all(r["passed"] for r in reports)}
print(json.dumps(report, indent=2))
raise SystemExit(0 if report["passed"] else 1)
