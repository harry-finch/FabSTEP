"""Generate two planar stops around a blind-pocket fillet and verify their STEP."""

import argparse
import json
import math
from pathlib import Path
import runpy
import subprocess

import FreeCAD as App
import MeshPart
import Part

base = runpy.run_path(
    str(Path(__file__).with_name("validate-oblique-pocket-freecad.py"))
)
V, near = App.Vector, base["near"]
CENTER_X, CENTER_Z = 7.5, 6.5
RADIUS = 2.5


def reference(slopes):
    blank = Part.makeBox(40, 50, 11, V(-12, -5, 0))
    cutter = Part.makeBox(14, 48, 14, V(-4, -4, 4))
    (edge,) = [e for e in cutter.Edges if (e.CenterOfMass - V(10, 20, 4)).Length < 1e-8]
    cutter = cutter.makeFillet(RADIUS, [edge])
    for station, (sx, sz) in zip((6, 29), slopes):
        corners = [
            V(x, station + sx * (x - CENTER_X) + sz * (z - CENTER_Z), z)
            for x, z in [(-30, -30), (30, -30), (30, 30), (-30, 30)]
        ]
        face = Part.Face(Part.makePolygon(corners + corners[:1]))
        cutter = cutter.common(face.extrude(V(0, 100 if station == 6 else -100, 0)))
    solid = blank.cut(cutter).removeSplitter()
    if not solid.isValid() or not solid.isClosed():
        raise ValueError("Invalid pocket drawing")
    return solid


def inspect(shape, volume, slopes):
    if len(shape.Solids) != 1 or not shape.isValid() or not shape.isClosed():
        raise ValueError("Expected one closed solid")
    near(shape.Volume, volume, 0.001)
    near(shape.BoundBox.XLength, 40)
    near(shape.BoundBox.YLength, 50)
    near(shape.BoundBox.ZLength, 11)
    cylinders = [f for f in shape.Faces if isinstance(f.Surface, Part.Cylinder)]
    if len(cylinders) != 1 or len(shape.Faces) != 12:
        raise ValueError(
            f"Expected 11 planes and one cylinder, got {len(shape.Faces)} faces"
        )
    wall = cylinders[0]
    near(wall.Surface.Radius, RADIUS)
    if sum(isinstance(f.Surface, Part.Plane) for f in shape.Faces) != 11:
        raise ValueError("Expected eleven planes")
    ends = sorted(
        (e for e in wall.Edges if isinstance(e.Curve, Part.Ellipse)),
        key=lambda e: e.Curve.Center.y,
    )
    if len(ends) != 2 or len(wall.Edges) != 4:
        raise ValueError("Expected a pair of elliptic planar stops and two rails")
    for e, station, (sx, sz) in zip(ends, (6, 29), slopes):
        center = V(CENTER_X, station, CENTER_Z)
        near((e.Curve.Center - center).Length, 0.0)
        near(e.Curve.MajorRadius, RADIUS * math.sqrt(1 + sx * sx + sz * sz))
        near(e.Curve.MinorRadius, RADIUS)
        owners = shape.ancestorsOfType(e, Part.Face)
        others = [f for f in owners if not wall.isSame(f)]
        if len(owners) != 2 or len(others) != 1:
            raise ValueError("Stop edge must share exactly one planar neighbor")
        normal = V(-sx, 1, -sz)
        normal.normalize()
        base["check_plane"](others[0], center, normal)
        for f in owners:
            p = base["ellipse_pcurve_report"](e, f)
            if p["max_sampled_mm"] > 2e-5:
                raise ValueError("Stop pcurve does not coincide with the shared edge")
        for x, z in [(1.0, 6.0), (8.0, 10.0)]:
            y = station + sx * (x - CENTER_X) + sz * (z - CENTER_Z)
            for dy in [-0.1, 0.1]:
                if shape.isInside(V(x, y + dy, z), 1e-7, False) != (
                    (station == 6) == (dy < 0)
                ):
                    raise ValueError("Material probe crossed the wrong stop")
    return {
        "faces": len(shape.Faces),
        "volume_mm3": shape.Volume,
        "elliptic_stops": len(ends),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--cli", type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    report = {}
    for case, slopes, rotation, mirror, fineness in [
        ("opposed", ((-0.25, 0.0), (0.5, 0.0)), False, False, 0.18),
        ("parallel", ((0.5, 0.25), (0.5, 0.25)), False, True, 0.13),
        ("compound", ((-0.25, 0.4), (0.5, -0.2)), True, False, 0.13),
    ]:
        solid = reference(slopes)
        volume = solid.Volume
        pose = (
            App.Placement(V(0, 0, 0), App.Rotation(V(0, 0, 1), 17))
            if rotation
            else App.Placement()
        )
        if mirror:
            solid = solid.mirror(V(0, 0, 0), V(1, 0, 0))
        solid.Placement = pose.multiply(solid.Placement)
        ref = args.output / f"{case}-ref.step"
        solid.exportStep(str(ref))
        mesh = MeshPart.meshFromShape(
            Shape=Part.read(str(ref)),
            LinearDeflection=0.02,
            AngularDeflection=fineness,
            Relative=False,
        )
        stl = args.output / f"{case}.stl"
        mesh.write(str(stl))
        targets = [ref]
        if args.cli:
            step = args.output / f"{case}-auto.step"
            with step.with_suffix(".json").open("w") as log:
                subprocess.run(
                    [
                        str(args.cli.resolve()),
                        "convert",
                        str(stl),
                        "-o",
                        str(step),
                        "--force",
                        "--mode",
                        "auto",
                        "--unit",
                        "mm",
                        "--json",
                    ],
                    stdout=log,
                    check=True,
                )
            targets.append(step)
        for target in targets:
            rebuilt = Part.read(str(target))
            rebuilt.Placement = pose.inverse().multiply(rebuilt.Placement)
            if mirror:
                rebuilt = rebuilt.mirror(V(0, 0, 0), V(1, 0, 0))
            report[target.stem] = dict(
                triangles=mesh.CountFacets, **inspect(rebuilt, volume, slopes)
            )
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
