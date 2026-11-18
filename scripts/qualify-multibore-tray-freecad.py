"""Qualify independent floor and lateral bores in a tray with eight rounded corners."""

import argparse
import json
import math
from pathlib import Path
import subprocess

import FreeCAD as App
import MeshPart
import Part

V = App.Vector
BORES = [
    (V(-6, 3, 0), V(0, 0, 1), 1.5, 3.0),
    (V(6, -2, 0), V(0, 0, 1), 2.0, 3.0),
    (V(3, -15, 7), V(0, 1, 0), 1.6, 2.0),
]


def rounded_box(w, d, h, z, r):
    box = Part.makeBox(w, d, h, V(-w / 2, -d / 2, z))
    rails = [
        e
        for e in box.Edges
        if abs(e.Vertexes[0].Point.z - e.Vertexes[1].Point.z) > h - 0.001
    ]
    return box.makeFillet(r, rails)


def drawing():
    tray = rounded_box(40, 30, 12, 0, 4).cut(rounded_box(36, 26, 12, 3, 2))
    for start, direction, r, depth in BORES:
        tray = tray.cut(Part.makeCylinder(r, depth + 2, start - direction, direction))
    return tray.removeSplitter()


def inspect(path):
    solid = Part.read(str(path))
    if len(solid.Solids) != 1 or not solid.isClosed() or not solid.isValid():
        raise ValueError("Open or invalid tray")
    walls = [f for f in solid.Faces if isinstance(f.Surface, Part.Cylinder)]
    planes = [f for f in solid.Faces if isinstance(f.Surface, Part.Plane)]
    if len(solid.Faces) != 22 or len(walls) != 11 or len(planes) != 11:
        raise ValueError("Expected eight corner fillets, three bores and eleven planes")
    expected = (
        (40 * 30 - (4 - math.pi) * 16) * 12
        - (36 * 26 - (4 - math.pi) * 4) * 9
        - sum(math.pi * r * r * d for _, _, r, d in BORES)
    )
    if abs(solid.Volume - expected) > 0.001:
        raise ValueError("Tray volume mismatch")
    for origin, axis, r, depth in BORES:
        (wall,) = [
            f
            for f in walls
            if abs(f.Surface.Radius - r) < 2e-5
            and abs(f.Surface.Axis.dot(axis)) > 1 - 1e-8
            and (f.Surface.Center - origin).cross(axis).Length < 2e-5
        ]
        mouths = []
        for e in wall.Edges:
            if not isinstance(e.Curve, Part.Circle):
                continue
            owners = solid.ancestorsOfType(e, Part.Face)
            if (
                not e.isClosed()
                or len(owners) != 2
                or sum(isinstance(f.Surface, Part.Plane) for f in owners) != 1
            ):
                raise ValueError("Bore lacks a closed circle shared with a plane")
            mouths.append((e.Curve.Center - origin).dot(axis))
        if (
            len(mouths) != 2
            or max(abs(a - b) for a, b in zip(sorted(mouths), [0.0, depth])) > 2e-5
        ):
            raise ValueError("Incorrect bore depth")
        for t in [0.1, depth / 2, depth - 0.1]:
            point = origin + axis * t
            if solid.isInside(point, 1e-7, False) or not solid.isInside(
                point + V(r + 0.1, 0, 0), 1e-7, False
            ):
                raise ValueError("Bore material mismatch")
    return {
        "faces": len(solid.Faces),
        "solids": 1,
        "volume_mm3": solid.Volume,
        "bores": 3,
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--cli", type=Path)
    args = parser.parse_args()
    root = args.directory
    root.mkdir(parents=True, exist_ok=True)
    ref = root / "tray-design.step"
    drawing().exportStep(str(ref))
    report = {"design": inspect(ref)}
    for label, angle in [("coarse", 0.2), ("fine", 0.13)]:
        m = MeshPart.meshFromShape(
            Shape=Part.read(str(ref)),
            LinearDeflection=0.03,
            AngularDeflection=angle,
            Relative=False,
        )
        stl = root / (label + ".stl")
        m.write(str(stl))
        if args.cli:
            dest = stl.with_suffix(".step")
            with dest.with_suffix(".json").open("w") as out:
                subprocess.run(
                    [
                        args.cli.resolve(),
                        "convert",
                        stl,
                        "--mode",
                        "auto",
                        "--unit",
                        "mm",
                        "--force",
                        "--json",
                        "-o",
                        dest,
                    ],
                    stdout=out,
                    check=True,
                )
            report[label] = dict(triangles=m.CountFacets, **inspect(dest))
    print(json.dumps(report, indent=2))
