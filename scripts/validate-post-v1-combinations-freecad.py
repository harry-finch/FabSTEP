"""Independent FreeCAD reimport of the cone/open-notch STEP outputs.

PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
 /Applications/FreeCAD.app/Contents/Resources/bin/python \
 scripts/validate-post-v1-combinations-freecad.py OUTPUT_DIRECTORY

The analytic Part reference is built independently from nominal dimensions.
This script reads CLI/native exports; it never overwrites them.
"""
import json
import math
from pathlib import Path
import sys
from collections import Counter

import FreeCAD as App
import Part

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = json.loads((ROOT / "fixtures/post_v1_combinations/manifest.json").read_text())
V = App.Vector


def transform(point, variant):
    if variant == "mirrored":
        return V(55-point[0], point[1]-17, point[2]+3)
    if variant == "rotated":
        return App.Rotation(V(1,2,3), math.degrees(.63)).multVec(V(*point)) + V(13,-7,11)
    return V(*point)


def close(a, b):
    return math.isclose(a, b, rel_tol=1e-8, abs_tol=1e-6)


def inspect(path, expected):
    shape = Part.read(str(path))
    surfaces = Counter(type(face.Surface).__name__ for face in shape.Faces)
    cylinders = sorted((face.Surface.Radius, abs(face.ParameterRange[1]-face.ParameterRange[0]), face.Area)
                       for face in shape.Faces if isinstance(face.Surface, Part.Cylinder))
    cones = [face for face in shape.Faces if isinstance(face.Surface, Part.Cone)]
    box = shape.BoundBox
    bounds = [[box.XMin,box.YMin,box.ZMin],[box.XMax,box.YMax,box.ZMax]]
    checks = {
        "closed_valid_solid": shape.isClosed() and shape.isValid() and len(shape.Solids) == 1,
        "surface_counts": surfaces == {"Plane":7,"Cylinder":2,"Cone":1},
        "volume": close(shape.Volume,expected["volume_mm3"]),
        "bounds": all(close(a,b) for row,nominal in zip(bounds,expected["bounds_mm"]) for a,b in zip(row,nominal)),
        "cylinder_radii_domains_areas": len(cylinders) == 2 and all(
            close(a,b) for actual,nominal in zip(cylinders,[(3,2*math.pi,36*math.pi),(4,math.pi,40*math.pi)])
            for a,b in zip(actual,nominal)),
        "cone_angle_area": len(cones) == 1 and close(abs(cones[0].Surface.SemiAngle),math.atan2(3,4)) and close(cones[0].Area,45*math.pi),
        "material_void_probes": all(shape.isInside(transform(point,expected["transform"]),1e-6,False) == inside
            for point,inside in [((1,12,5),False),((5,12,5),True),((32,12,5),False),
                                 ((36,12,9),False),((37.5,12,9),True),((36,12,7),True),((10,3,5),True)]),
    }
    return {"file":str(path),"volume_mm3":shape.Volume,"bounds_mm":bounds,
            "surfaces":dict(surfaces),"cylinders_radius_sweep_area":cylinders,
            "checks":checks,"passed":all(checks.values())}


def main():
    directory = Path(sys.argv[1])
    reference = Part.makeBox(44,24,10).cut(Part.makeCylinder(4,10,V(0,12,0))).cut(
        Part.makeCylinder(3,6,V(32,12,0)).fuse(Part.makeCone(3,6,4,V(32,12,6)))).removeSplitter()
    reference_volume = MANIFEST["cases"]["countersink_notch"]["volume_mm3"]
    reference_passed = reference.isValid() and reference.isClosed() and close(reference.Volume,reference_volume)
    reports = [inspect(directory / f"{name}.step",MANIFEST["cases"][name]) for name in (
        "countersink_notch","countersink_notch_rotated","countersink_notch_mirrored")]
    result = {"freecad":".".join(App.Version()[:3]),"reference_volume_mm3":reference.Volume,
              "reference_passed":reference_passed,"exports":reports,
              "passed":reference_passed and all(report["passed"] for report in reports)}
    (directory / "freecad.json").write_text(json.dumps(result,indent=2)+"\n")
    print(json.dumps(result,indent=2))
    raise SystemExit(0 if result["passed"] else 1)


if __name__ == "__main__":
    main()
