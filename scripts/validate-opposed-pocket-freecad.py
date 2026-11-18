"""Build and inspect a FreeCAD pocket with two opposed straight quarter fillets.

Run with FreeCAD's Python, passing an output directory and --cli PATH.
"""

import argparse
from collections import Counter
import json
import math
from pathlib import Path
import subprocess

import FreeCAD as App
import MeshPart
import Part


V = App.Vector
FLOOR = 3.0
ENDS = (1.0, 24.0)
LENGTH = ENDS[1] - ENDS[0]
# Wall X, direction from the wall to the opening, fillet radius.
SIDES = ((-5.0, 1.0, 1.75), (12.0, -1.0, 3.25))
BOUNDS = (-12.0, -7.0, -2.0, 34.0, 31.0, 12.0)
VOLUME = (
    46 * 38 * 14
    - LENGTH * 17 * (12 - FLOOR)
    + LENGTH * (1 - math.pi / 4) * sum(radius**2 for _, _, radius in SIDES)
)


def reference():
    blank = Part.makeBox(46, 38, 14, V(*BOUNDS[:3]))
    cutter = Part.makeBox(17, LENGTH, 15, V(SIDES[0][0], ENDS[0], FLOOR))
    for x, _, radius in SIDES:
        midpoint = V(x, sum(ENDS) / 2, FLOOR)
        (edge,) = [e for e in cutter.Edges if (e.CenterOfMass - midpoint).Length < 1e-8]
        cutter = cutter.makeFillet(radius, [edge])
    return blank.cut(cutter).removeSplitter()


def near(value, expected, tol=2e-5):
    if not math.isclose(value, expected, abs_tol=tol, rel_tol=0):
        raise ValueError(f"Expected {expected} ± {tol}, got {value}")


def check_material(solid):
    probes = [
        ((0, 12, FLOOR - 0.1), True),
        ((0, 12, FLOOR + 0.1), False),
        ((0, ENDS[0] - 0.1, 7), True),
        ((0, ENDS[0] + 0.1, 7), False),
        ((0, ENDS[1] - 0.1, 7), False),
        ((0, ENDS[1] + 0.1, 7), True),
    ]
    for x, direction, radius in SIDES:
        for offset, material in [(0.12, True), (-0.12, False)]:
            distance = radius - (radius + offset) / math.sqrt(2)
            probes.append(((x + direction * distance, 12, FLOOR + distance), material))
    for point, material in probes:
        if solid.isInside(V(*point), 1e-7, False) != material:
            raise ValueError(f"Incorrect material at {point}")


def inspect_fillet(solid, face, side):
    x, direction, radius = side
    cx = x + direction * radius
    cz = FLOOR + radius
    surf = face.Surface
    near(surf.Radius, radius)
    near(surf.Center.x, cx)
    near(surf.Center.z, cz)
    near(abs(surf.Axis.y), 1, 1e-8)
    if face.Orientation != "Reversed":
        raise ValueError("Expected a concave wall")
    u0, u1, v0, v1 = face.ParameterRange
    near(u1 - u0, math.pi / 2)
    near(v1 - v0, LENGTH)
    neighbors = []
    neighbor_faces = []
    for edge in face.Edges:
        (other,) = [
            f for f in solid.ancestorsOfType(edge, Part.Face) if not f.isSame(face)
        ]
        if not isinstance(other.Surface, Part.Plane):
            raise ValueError("Expected a planar neighbor")
        axis = other.Surface.Axis
        i = max(range(3), key=lambda j: abs(axis[j]))
        near(abs(axis[i]), 1.0, 1e-8)
        if isinstance(edge.Curve, Part.Circle):
            kind = "arc"
            near(edge.Curve.Radius, radius)
            near(edge.Curve.Center.x, cx)
            near(edge.Curve.Center.z, cz)
            near(edge.Length, radius * math.pi / 2)
        elif isinstance(edge.Curve, Part.Line):
            kind = "rail"
            near(edge.Length, LENGTH)
        else:
            raise ValueError(f"Unexpected fillet edge {edge.Curve}")
        neighbors.append((kind, i, other.CenterOfMass[i]))
        neighbor_faces.append(other)
    expected = [("arc", 1, y) for y in ENDS] + [("rail", 0, x), ("rail", 2, FLOOR)]
    if len(neighbors) != 4:
        raise ValueError("Expected four shared edges")
    for item, target in zip(sorted(neighbors), expected):
        if item[:2] != target[:2]:
            raise ValueError(f"Unexpected fillet boundary {item}")
        near(item[2], target[2])
    return dict(radius_mm=surf.Radius, neighbors=sorted(neighbors)), neighbor_faces


def inspect(path):
    solid = Part.read(str(path))
    if len(solid.Solids) != 1 or not solid.isValid() or not solid.isClosed():
        raise ValueError(f"{path} is not a single valid closed solid")
    box = solid.BoundBox
    bounds = (box.XMin, box.YMin, box.ZMin, box.XMax, box.YMax, box.ZMax)
    for actual, expected in zip(bounds, BOUNDS):
        near(actual, expected)
    check_material(solid)
    faces = Counter(type(f.Surface).__name__ for f in solid.Faces)
    report = dict(faces=faces, volume_mm3=solid.Volume, bounds_mm=bounds)
    if faces != {"Plane": 11, "Cylinder": 2}:
        raise ValueError(
            f"{path}: expected eleven planes and two cylinders, got {faces}"
        )
    near(solid.Volume, VOLUME, 3e-4)
    walls = sorted(
        (f for f in solid.Faces if isinstance(f.Surface, Part.Cylinder)),
        key=lambda f: f.Surface.Center.x,
    )
    first, left = inspect_fillet(solid, walls[0], SIDES[0])
    second, right = inspect_fillet(solid, walls[1], SIDES[1])
    # The two ends and floor must each be one face shared by both fillets.
    shared = [a for a in left if any(a.isSame(b) for b in right)]
    if len(shared) != 3:
        raise ValueError("The opposed fillets must share both stops and the floor")
    report["fillets"] = [first, second]
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--cli", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(exist_ok=True, parents=True)
    source = reference()
    step = args.output / "reference.step"
    source.exportStep(str(step))
    report = dict(
        freecad=".".join(App.Version()[:3]), reference=inspect(step), cases={}
    )
    for name, angular in [("coarse", 0.25), ("fine", 0.10)]:
        mesh = MeshPart.meshFromShape(
            Shape=source,
            LinearDeflection=0.025,
            AngularDeflection=angular,
            Relative=False,
        )
        stl = args.output / f"{name}.stl"
        mesh.write(str(stl))
        case = dict(triangles=mesh.CountFacets)
        step = args.output / f"{name}.step"
        with (args.output / f"{name}-conversion.json").open("w") as log:
            subprocess.run(
                [
                    str(args.cli.resolve()),
                    "convert",
                    str(stl),
                    "--unit",
                    "mm",
                    "--mode",
                    "auto",
                    "--force",
                    "--json",
                    "-o",
                    str(step),
                ],
                check=True,
                stdout=log,
            )
        case.update(inspect(step))
        report["cases"][name] = case
    print(json.dumps(report, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
