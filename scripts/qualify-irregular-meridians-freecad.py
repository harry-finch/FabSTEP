"""Draw smooth meridians in FreeCAD, mesh their solids and inspect FabSTEP exports."""

import argparse
import math
from pathlib import Path
import FreeCAD as App
import MeshPart
import Part

V = App.Vector
CASES = {
    "bulged": ([5.0, 9.0, 9.0, 5.0], 20.0),
    "inflected": ([4.0, 9.0, 3.0, 8.0], 24.0),
}


def draw(r, height):
    curve = Part.BezierCurve()
    curve.setPoles([V(v, 0, height * i / 3) for i, v in enumerate(r)])
    meridian = Part.Wire(
        [
            Part.makeLine(V(0, 0, 0), V(r[0], 0, 0)),
            curve.toShape(),
            Part.makeLine(V(r[-1], 0, height), V(0, 0, height)),
            Part.makeLine(V(0, 0, height), V(0, 0, 0)),
        ]
    )
    return Part.Face(meridian).revolve(V(0, 0, 0), V(0, 0, 1), 360)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("output", type=Path)
    p.add_argument("--mesh-dir", type=Path)
    p.add_argument("--check", action="store_true")
    args = p.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    if args.mesh_dir:
        args.mesh_dir.mkdir(parents=True, exist_ok=True)
    for name, (r, h) in CASES.items():
        drawing = draw(r, h)
        if name == "inflected":
            drawing.rotate(V(0, 0, 0), V(1, 0, 0), 90)
            drawing.translate(V(4.0, -8.0, 3.0))
        drawing.exportStep(str(args.output / f"{name}-reference.step"))
        if args.mesh_dir:
            for variant, angle in [("coarse", 0.2), ("fine", 0.11)]:
                ref = Part.read(str(args.output / f"{name}-reference.step"))
                m = MeshPart.meshFromShape(
                    Shape=ref,
                    LinearDeflection=0.018,
                    AngularDeflection=angle,
                    Relative=False,
                )
                namepath = f"{name}-{variant}"
                m.write(str(args.mesh_dir / f"{namepath}.stl"))
                print(namepath, m.CountFacets)
        if args.check:
            for variant in ("coarse", "fine"):
                if name == "inflected" and variant == "fine":
                    continue  # Over the available radial bound.
                solid = Part.read(str(args.output / f"{name}-{variant}.step"))
                assert (
                    len(solid.Solids) == 1
                    and solid.isValid()
                    and solid.Solids[0].isClosed()
                )
                assert len(solid.Faces) == 3
                (wall,) = [
                    f
                    for f in solid.Faces
                    if isinstance(f.Surface, Part.SurfaceOfRevolution)
                ]
                (ref,) = [
                    f
                    for f in drawing.Faces
                    if isinstance(f.Surface, Part.SurfaceOfRevolution)
                ]
                u0, u1, v0, v1 = ref.ParameterRange
                # Compare the reconstructed support to points evaluated directly on the CAD drawing.
                maximum = max(
                    wall.distToShape(
                        Part.Vertex(
                            ref.valueAt(u0 + i * (u1 - u0) / 5, v0 + j * (v1 - v0) / 32)
                        )
                    )[0]
                    for i in range(5)
                    for j in range(33)
                )
                assert maximum < 2e-5
                assert math.isclose(solid.Volume, drawing.Volume, rel_tol=2e-6)
                print(name, variant, solid.Volume, maximum)


if __name__ == "__main__":
    main()
