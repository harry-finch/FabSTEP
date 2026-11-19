"""Draw smooth meridians in FreeCAD, mesh their solids and inspect FabSTEP exports."""

import argparse
import math
import runpy
from pathlib import Path
import FreeCAD as App
import MeshPart
import Part

V = App.Vector
CASES = {
    "bulged": ([5.0, 9.0, 9.0, 5.0], 20.0),
    "inflected": ([4.0, 9.0, 3.0, 8.0], 24.0),
    "two_span": ([6.0, 4.5, 7.5, 8.0, 8.0 + 5.0 / 6.0, 4.0, 6.0], 24.0),
    "two_span_turned": ([8.0, 11.0, 7.0, 6.0, 6.0 - 2.0 / 3.0, 8.5, 7.5], 30.0),
    "kinked": ([6.0, 4.5, 7.5, 8.0, 7.0, 4.0, 6.0], 24.0),
}


def draw(name, r, height):
    if len(r) == 4:
        curve = Part.BezierCurve()
        curve.setPoles([V(v, 0, height * i / 3) for i, v in enumerate(r)])
    else:
        k = 0.6 if name == "two_span_turned" else 3 / 8
        ts = [0, k / 3, 2 * k / 3, k, k + (1 - k) / 3, k + 2 * (1 - k) / 3, 1]
        poles = [V(v, 0, t * height) for t, v in zip(ts, r)]
        multiplicity = 3 if name == "kinked" else 2
        if multiplicity == 2:
            poles.pop(3)
        curve = Part.BSplineCurve()
        curve.buildFromPolesMultsKnots(
            poles, [4, multiplicity, 4], [0.0, k, 1.0], False, 3
        )
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
        drawing = draw(name, r, h)
        if name == "inflected":
            drawing.rotate(V(0, 0, 0), V(1, 0, 0), 90)
            drawing.translate(V(4.0, -8.0, 3.0))
        if name == "two_span_turned":
            drawing.rotate(V(0, 0, 0), V(1, 0, 0), 180)
            drawing.translate(V(1.0, 3.0, -2.0))
        drawing.exportStep(str(args.output / f"{name}-reference.step"))
        if args.mesh_dir:
            for variant, angle in [("coarse", 0.2), ("fine", 0.11)]:
                if name == "kinked" and variant == "fine":
                    continue
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
                if name == "kinked" or (name == "inflected" and variant == "fine"):
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
                measured = solid.Volume
                if len(r) == 7:
                    # Generic OCCT integration can step over the internal knot.
                    # Reuse the independent per-span quadrature on the exported STEP.
                    turned = name == "two_span_turned"
                    k = 0.6 if turned else 3 / 8
                    volume = (
                        math.pi
                        * h
                        * sum(
                            w
                            * a
                            * b
                            * math.comb(3, i)
                            * math.comb(3, j)
                            / math.comb(6, i + j)
                            / 7
                            for w, part in ((k, r[:4]), (1 - k, r[3:]))
                            for i, a in enumerate(part)
                            for j, b in enumerate(part)
                        )
                    )
                    case = {
                        "parameters": {
                            "height_mm": h,
                            "radii_mm": r[:4],
                            "axis": [0, 0, -1 if turned else 1],
                            "origin_mm": [1, 3, -2] if turned else [0, 0, 0],
                            "volume_mm3": volume,
                        },
                        "second_span": {"start_fraction": k, "next_radii_mm": r[4:]},
                        "material_probes": [],
                    }
                    verify = runpy.run_path(
                        str(Path(__file__).with_name("validate-cubic-spans-freecad.py"))
                    )["check"]
                    measured = verify(case, args.output / f"{name}-{variant}.step")[
                        "volume_mm3"
                    ]
                print(name, variant, measured, maximum)


if __name__ == "__main__":
    main()
