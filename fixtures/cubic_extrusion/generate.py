"""Make analytic reference solids and binary STL cases with FreeCAD Part."""

import argparse
import struct
from pathlib import Path

import FreeCAD as App
import Part


def generate(root, exports):
    v = App.Vector
    root.mkdir(parents=True, exist_ok=True)
    for name, curve, length, floor, bore in [
        ("cam", [0, 3.2, -1.4, 0], 12, -4, None),
        ("guide", [0, -1, 2.5, 0], 16, -6, (7, -3.2, 1.1)),
    ]:
        poles = [v(length * i / 3, y, 0) for i, y in enumerate(curve)]
        spline = Part.BezierCurve()
        spline.setPoles(poles)
        outer = Part.Wire(
            [
                spline.toShape(),
                Part.makeLine(poles[-1], v(length, floor, 0)),
                Part.makeLine(v(length, floor, 0), v(0, floor, 0)),
                Part.makeLine(v(0, floor, 0), poles[0]),
            ]
        )
        base = Part.Face(outer)
        if bore:
            cx, cy, r = bore
            hole = Part.Wire([Part.makeCircle(r, v(cx, cy, 0))])
            base = base.cut(Part.Face(hole))
        solid = base.extrude(v(0, 0, 5))
        solid.exportStep(str(root / f"{name}.step"))
        for density, deflection in [("coarse", 0.024), ("fine", 0.008)]:
            if exports:
                actual = Part.read(str(exports / f"{name}-{density}.step"))
                assert (
                    actual.isValid()
                    and len(actual.Solids) == 1
                    and actual.Solids[0].isClosed()
                )
                assert (
                    len(actual.Faces) == 6 + int(bore is not None)
                    and abs(actual.Volume - solid.Volume) < 1e-4
                )
                (flank,) = [
                    f
                    for f in actual.Faces
                    if isinstance(f.Surface, Part.SurfaceOfExtrusion)
                ]
                for j in range(49):
                    profile = spline.value(j / 48)
                    for z in (0.0, 1.25, 2.5, 3.75, 5.0):
                        assert (
                            flank.distToShape(Part.Vertex(profile + v(0, 0, z)))[0]
                            < 3e-5
                        )
                if bore:
                    cx, cy, r = bore
                    assert not actual.isInside(v(cx, cy, 2.5), 1e-7, True)
                    assert actual.isInside(v(cx + r + 0.1, cy, 2.5), 1e-7, True)
                print(name, density, "245 wall points, volume", actual.Volume)
                continue
            pts, tri = solid.tessellate(deflection)
            with (root / f"{name}-{density}.stl").open("wb") as f:
                f.write(bytes(80) + struct.pack("<I", len(tri)))
                for ids in tri:
                    f.write(
                        struct.pack(
                            "<12fH", 0, 0, 0, *(p for i in ids for p in pts[i]), 0
                        )
                    )
            print(name, density, len(tri), solid.Volume)
            if hasattr(solid, "cleanTriangulation"):
                solid.cleanTriangulation()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("directory", type=Path)
    parser.add_argument("--check-exports", type=Path)
    args = parser.parse_args()
    generate(args.directory, args.check_exports)
