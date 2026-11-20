"""Reference of a turned shaft with two lands and an axial undercut."""

import argparse
import math
from pathlib import Path

import FreeCAD as App
import MeshPart
import Part


def generate(directory, exports):
    directory.mkdir(exist_ok=True, parents=True)
    v = App.Vector
    curve = Part.BezierCurve()
    curve.setPoles(
        [v(r, 0, z) for r, z in zip((3.0, 5.5, 8.0, 10.5), (0.0, 13.0, -5.0, 8.0))]
    )
    a, b = v(0, 0, -2.0), v(0, 0, 11.0)
    start, end = v(3, 0, -2.0), v(10.5, 0, 11.0)
    wire = Part.Wire(
        [
            Part.makeLine(a, start),
            Part.makeLine(start, curve.value(0)),
            curve.toShape(),
            Part.makeLine(curve.value(1), end),
            Part.makeLine(end, b),
            Part.makeLine(b, a),
        ]
    )
    solid = Part.Face(wire).revolve(v(), v(0, 0, 1), 360)
    assert len(solid.Faces) == 5 and solid.isValid()
    solid.exportStep(str(directory / "reference.step"))
    for name, linear, angle in [("coarse", 0.009, 0.18), ("fine", 0.007, 0.13)]:
        if exports:
            actual = Part.read(str(exports / f"shaft-{name}.step"))
            assert (
                actual.isValid()
                and len(actual.Solids) == 1
                and actual.Solids[0].isClosed()
            )
            assert len(actual.Faces) == 5 and abs(actual.Volume - solid.Volume) < 2e-5
            (wall,) = [
                f
                for f in actual.Faces
                if isinstance(f.Surface, Part.SurfaceOfRevolution)
            ]
            for i in range(5):
                angle = 2 * math.pi * i / 5
                for j in range(49):
                    profile = curve.value(j / 48)
                    point = v(
                        profile.x * math.cos(angle),
                        profile.x * math.sin(angle),
                        profile.z,
                    )
                    assert wall.distToShape(Part.Vertex(point))[0] < 3e-5
                for t in (0.17, 0.31, 0.5, 0.69, 0.83):
                    profile = curve.value(t)
                    for sign in (-1, 1):
                        check = v(
                            profile.x * math.cos(angle),
                            profile.x * math.sin(angle),
                            profile.z + sign * 0.04,
                        )
                        inside = sign > 0
                        assert solid.isInside(check, 1e-7, True) == inside
                        assert actual.isInside(check, 1e-7, True) == inside
            print(name, "245 wall points, 50 material probes, volume", actual.Volume)
            continue
        reopened = Part.read(str(directory / "reference.step"))
        mesh = MeshPart.meshFromShape(
            Shape=reopened,
            LinearDeflection=linear,
            AngularDeflection=angle,
            Relative=False,
        )
        mesh.write(str(directory / f"shaft-{name}.stl"))
        print(name, mesh.CountFacets, solid.Volume)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("directory", type=Path)
    parser.add_argument("--check-exports", type=Path)
    args = parser.parse_args()
    generate(args.directory, args.check_exports)
