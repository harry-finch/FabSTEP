"""Build a second pocket in FreeCAD, convert its STL in Auto and inspect the STEP.

Run with FreeCAD's Python. Arguments are the fabstep CLI and an output directory.
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


RADIUS = 2.5
VOLUME = 40 * 50 * 11 - 14 * 25 * 7 + 25 * RADIUS**2 * (1 - math.pi / 4)


def reference():
    blank = Part.makeBox(40, 50, 11, App.Vector(-12, -5, 0))
    cutter = Part.makeBox(14, 25, 14, App.Vector(-4, 6, 4))
    (edge,) = [
        edge
        for edge in cutter.Edges
        if abs(edge.CenterOfMass.x - 10) < 1e-8
        and abs(edge.CenterOfMass.z - 4) < 1e-8
        and abs(edge.Length - 25) < 1e-8
    ]
    solid = blank.cut(cutter.makeFillet(RADIUS, [edge])).removeSplitter()
    if not solid.isValid() or abs(solid.Volume - VOLUME) > 1e-7:
        raise ValueError("Invalid independent pocket reference")
    return solid


def inspect(path):
    solid = Part.read(str(path))
    faces = Counter(type(face.Surface).__name__ for face in solid.Faces)
    if not (
        solid.isValid()
        and solid.isClosed()
        and len(solid.Solids) == 1
        and faces == {"Plane": 11, "Cylinder": 1}
    ):
        raise ValueError("Expected a closed pocket with one cylindrical blend")
    (cylinder,) = [
        face for face in solid.Faces if isinstance(face.Surface, Part.Cylinder)
    ]
    u0, u1, v0, v1 = cylinder.ParameterRange
    if (
        cylinder.Orientation != "Reversed"
        or abs(cylinder.Surface.Radius - RADIUS) > 2e-5
        or abs(u1 - u0 - math.pi / 2) > 2e-5
        or abs(v1 - v0 - 25) > 2e-5
        or abs(solid.Volume - VOLUME) > 2e-4
    ):
        raise ValueError("Incorrect concavity, radius, cylindrical trim or volume")
    neighbors = []
    for edge in cylinder.Edges:
        (other,) = [
            face
            for face in solid.ancestorsOfType(edge, Part.Face)
            if not face.isSame(cylinder)
        ]
        axis = other.Surface.Axis
        coordinate = max(range(3), key=lambda i: abs(axis[i]))
        if abs(abs(axis[coordinate]) - 1) > 1e-8:
            raise ValueError("Unexpected support plane")
        neighbors.append(
            (type(edge.Curve).__name__, coordinate, other.CenterOfMass[coordinate])
        )
    expected = [("Circle", 1, 6), ("Circle", 1, 31), ("Line", 0, 10), ("Line", 2, 4)]
    if len(neighbors) != len(expected) or any(
        a[:2] != b[:2] or abs(a[2] - b[2]) > 2e-5
        for a, b in zip(sorted(neighbors), expected)
    ):
        raise ValueError(
            "The blend must stop at both walls and join the floor and side"
        )
    for point, material in [
        ((9.9, 18, 4.1), True),
        ((9, 18, 6), False),
        ((9, 5.9, 6), True),
        ((9, 31.1, 6), True),
        ((0, 18, 4.15), False),
        ((0, 18, 3.9), True),
    ]:
        if solid.isInside(App.Vector(*point), 1e-6, False) != material:
            raise ValueError(f"Unexpected material at {point}")
    return dict(
        faces=len(solid.Faces),
        volume_mm3=solid.Volume,
        radius_mm=cylinder.Surface.Radius,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cli", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    source = reference()
    reports = {}
    for name, angular in [("coarse", 0.25), ("fine", 0.10)]:
        mesh = MeshPart.meshFromShape(
            Shape=source,
            LinearDeflection=0.025,
            AngularDeflection=angular,
            Relative=False,
        )
        stl, step = (args.output / f"{name}.{suffix}" for suffix in ("stl", "step"))
        mesh.write(str(stl))
        with (args.output / f"{name}-conversion.json").open("w") as report:
            subprocess.run(
                [
                    str(args.cli.resolve()),
                    "convert",
                    str(stl),
                    "-o",
                    str(step),
                    "--unit",
                    "mm",
                    "--mode",
                    "auto",
                    "--json",
                ],
                stdout=report,
                check=True,
            )
        reports[name] = dict(triangles=mesh.CountFacets, **inspect(step))
    print(
        json.dumps(
            dict(
                freecad=".".join(App.Version()[:3]),
                reference_volume_mm3=VOLUME,
                results=reports,
            ),
            indent=2,
            allow_nan=False,
        )
    )


if __name__ == "__main__":
    main()
