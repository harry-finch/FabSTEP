"""Two adjacent bottom fillets of equal radius, at acute and obtuse side angles."""

import argparse
import json
import math
from pathlib import Path
import subprocess

import FreeCAD as App
import MeshPart
import Part

V = App.Vector
Z = V(0, 0, 1)
R = 2.5
L, W, FLOOR, DEPTH = 22.0, 19.0, 3.0, 11.0


def geometry(angle):
    a = math.radians(angle)
    x = V(1, 0, 0)
    b = V(math.cos(a), math.sin(a), 0)
    base = V(0, 0, FLOOR)
    far = V(L, (W - L * math.cos(a)) / math.sin(a), 0)
    loop = [base, base + x * L, base + far, base + b * W, base]
    return x, b, loop


def drawing(angle):
    _, _, loop = geometry(angle)
    cutter = Part.Face(Part.makePolygon(loop)).extrude(Z * (DEPTH + 1))
    edges = []
    for end in [loop[1], loop[3]]:
        midpoint = (end + loop[0]) * 0.5
        (edge,) = [e for e in cutter.Edges if (e.CenterOfMass - midpoint).Length < 1e-6]
        edges.append(edge)
    blank = Part.makeBox(74, 70, 14, V(-25, -10, 0))
    return blank.cut(cutter.makeFillet(R, edges)).removeSplitter()


def inspect(tray, angle):
    x, b, loop = geometry(angle)
    a = math.radians(angle)
    assert len(tray.Solids) == 1 and tray.isClosed() and tray.isValid()
    cylinders = [f for f in tray.Faces if isinstance(f.Surface, Part.Cylinder)]
    assert len(cylinders) == 2 and len(tray.Faces) == 13, (
        len(cylinders),
        len(tray.Faces),
    )
    (ellipse,) = [e for e in tray.Edges if isinstance(e.Curve, Part.Ellipse)]
    assert all(
        any(f.isSame(c) for f in tray.ancestorsOfType(ellipse, Part.Face))
        for c in cylinders
    )
    assert abs(ellipse.Curve.MajorRadius - R / math.sin(a / 2)) < 2e-5
    assert abs(ellipse.Curve.MinorRadius - R) < 2e-5
    normal0, normal1 = V(0, 1, 0), V(math.sin(a), -math.cos(a), 0)
    bisector = (x + b) / (x + b).Length
    m = math.sin(a / 2)
    for direction, n in [(x, normal0), (b, normal1)]:
        (cylinder,) = [
            f for f in cylinders if abs(abs(f.Surface.Axis.dot(direction)) - 1.0) < 1e-8
        ]
        assert abs(cylinder.Surface.Radius - R) < 2e-5
        # Match the 3D ellipse and both independent p-curves over the trim.
        curve, first, last = cylinder.curveOnSurface(ellipse)
        assert (
            abs(first - ellipse.FirstParameter) < 1e-8
            and abs(last - ellipse.LastParameter) < 1e-8
        )
        for k in range(257):
            u = first + (last - first) * k / 256
            uv = curve.value(u)
            assert (cylinder.valueAt(uv.x, uv.y) - ellipse.valueAt(u)).Length < 2e-5
        (circle,) = [e for e in cylinder.Edges if isinstance(e.Curve, Part.Circle)]
        assert len(tray.ancestorsOfType(circle, Part.Face)) == 2
        assert abs(circle.Curve.Radius - R) < 2e-5
    area = Part.Face(Part.makePolygon(loop)).Area
    volume = (
        74 * 70 * 14
        - area * DEPTH
        + (L + W) * (1 - math.pi / 4) * R**2
        - 1 / math.tan(a / 2) * (5 / 3 - math.pi / 2) * R**3
    )
    assert abs(volume - tray.Volume) < 0.001, (volume, tray.Volume)
    for p in [
        bisector * (R * 0.2 / m),
        bisector * (R * 0.5 / m),
        x * 9 + normal0 * 0.8,
        b * 9 + normal1 * 0.8,
    ]:
        nearest = min(normal0.dot(p), normal1.dot(p))
        z = FLOOR + R - math.sqrt(R**2 - (R - nearest) ** 2)
        for offset in [-0.04, 0.04]:
            assert tray.isInside(p + Z * (z + offset), 1e-7, False) == (offset < 0)
    return dict(
        faces=len(tray.Faces),
        volume_mm3=tray.Volume,
        ellipse_major=ellipse.Curve.MajorRadius,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--cli", type=Path)
    parser.add_argument("--require-analytic", action="store_true")
    args = parser.parse_args()
    root = args.directory
    root.mkdir(parents=True, exist_ok=True)
    for angle in [65, 115, 40]:
        ref = root / f"angle-{angle}-design.step"
        drawing(angle).exportStep(str(ref))
        shape = Part.read(str(ref))
        print(json.dumps(dict(angle=angle, design=inspect(shape, angle))), flush=True)
        for label, angle_def in [("coarse", 0.2), ("fine", 0.105)]:
            mesh = MeshPart.meshFromShape(
                Shape=shape,
                LinearDeflection=0.015,
                AngularDeflection=angle_def,
                Relative=False,
            )
            stl = root / f"angle-{angle}-{label}.stl"
            mesh.write(str(stl))
            if args.cli:
                dest = stl.with_suffix(".step")
                with dest.with_suffix(".json").open("w") as f:
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
                        stdout=f,
                        check=True,
                    )
                result = Part.read(str(dest))
                families = [f.Surface.TypeId for f in result.Faces]
                summary = dict(
                    angle=angle,
                    mesh=label,
                    triangles=mesh.CountFacets,
                    faces=len(families),
                    surface_types=sorted(set(families)),
                )
                if angle == 40:
                    assert (
                        result.isValid()
                        and result.isClosed()
                        and len(result.Solids) == 1
                    )
                    assert set(families) == {"Part::GeomPlane"}
                elif args.require_analytic or "Part::GeomCylinder" in families:
                    summary["analytic"] = inspect(result, angle)
                print(json.dumps(summary), flush=True)


if __name__ == "__main__":
    main()
