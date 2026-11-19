"""Design and independently read back a rectangular pocket with three coupled bottom fillets."""

import argparse
import json
import math
from pathlib import Path
import subprocess

import FreeCAD as App
import MeshPart
import Part

V = App.Vector
CASES = {"u": (22., 18., 2.5), "wide": (29., 15., 1.75), "narrow": (7., 18., 2.5)}
FLOOR = 3.
DEPTH = 11.


def drawing(w, length, r, unequal):
    blank = Part.makeBox(44, 36, FLOOR + DEPTH)
    cutter = Part.makeBox(w, length, DEPTH + 1, V(5, 6, FLOOR))
    edges = [e for e in cutter.Edges if abs(e.CenterOfMass.z - FLOOR) < 1e-8
             and abs(e.CenterOfMass.y - (6 + length)) > 1e-8]
    assert len(edges) == 3
    cutter = cutter.makeFillet(r, edges)
    solid = blank.cut(cutter).removeSplitter()
    if unequal:
        # Break only the left junction. The three-cylinder network then has
        # extra planar boundaries and cannot satisfy its incidence certificate.
        solid = solid.cut(Part.makeBox(r, r, DEPTH + 2, V(5 + r / 4, 6 + r / 4, FLOOR - 1))).removeSplitter()
        assert len(solid.Faces) > 14
    return solid


def inspect(solid, w, length, radius):
    assert solid.isValid() and solid.isClosed() and len(solid.Solids) == 1
    cylinders = [f for f in solid.Faces if isinstance(f.Surface, Part.Cylinder)]
    seams = [e for e in solid.Edges if isinstance(e.Curve, Part.Ellipse)]
    assert len(cylinders) == 3 and len(solid.Faces) == 14 and len(seams) == 2
    degree = []
    for face in cylinders:
        assert abs(face.Surface.Radius - radius) < 2e-5
        junctions = [e for e in seams if any(face.isSame(f) for f in solid.ancestorsOfType(e, Part.Face))]
        degree.append(len(junctions))
        for edge in junctions:
            assert len(solid.ancestorsOfType(edge, Part.Face)) == 2
            assert abs(edge.Curve.MajorRadius - math.sqrt(2) * radius) < 2e-5
            assert abs(edge.Curve.MinorRadius - radius) < 2e-5
            curve, a, b = face.curveOnSurface(edge)
            for j in range(65):
                t = a + (b - a) * j / 64
                q = curve.value(t)
                assert (face.valueAt(q.x, q.y) - edge.valueAt(t)).Length < 2e-5
    assert sorted(degree) == [1, 1, 2]
    circles = [e for e in solid.Edges if isinstance(e.Curve, Part.Circle)]
    assert len(circles) == 2
    # Integrate the rectangular cavity width and length over its lower quarter circle.
    volume = 44 * 36 * (DEPTH + FLOOR) - w * length * DEPTH + (2 * length + w) * (
        1 - math.pi / 4) * radius**2 - 2 * (5/3 - math.pi/2) * radius**3
    assert abs(solid.Volume - volume) < 0.001, (solid.Volume, volume)
    for x, y in [(0.4, 0.4), (w - 0.6, 0.6), (w / 2, 0.4), (0.4, length - 0.2)]:
        d = min(x, w - x, y)
        z = FLOOR + radius - math.sqrt(radius**2 - (radius - d)**2)
        for off in [-0.04, 0.04]:
            assert solid.isInside(V(5+x, 6+y, z+off), 1e-7, False) == (off < 0)
    return round(volume, 6)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("directory", type=Path)
    p.add_argument("--cli", type=Path)
    p.add_argument("--require-analytic", action="store_true")
    args = p.parse_args()
    args.directory.mkdir(exist_ok=True, parents=True)
    for name, (w, length, radius) in {**CASES, "notched": CASES["u"]}.items():
        notched = name == "notched"
        ref = args.directory / f"{name}-design.step"
        drawing(w, length, radius, notched).exportStep(str(ref))
        design = Part.read(str(ref))
        expected = None if notched else inspect(design, w, length, radius)
        for tier, angular in [("coarse", 0.2), ("fine", 0.105)]:
            mesh = MeshPart.meshFromShape(Shape=design, LinearDeflection=0.015,
                    AngularDeflection=angular, Relative=False)
            stl = args.directory / f"{name}-{tier}.stl"
            mesh.write(str(stl))
            row = dict(model=name, density=tier, triangles=mesh.CountFacets, volume_mm3=expected)
            if args.cli:
                dest = stl.with_suffix(".step")
                proc = subprocess.run([str(args.cli.resolve()), "convert", str(stl), "--mode", "auto",
                        "--unit", "mm", "--force", "--json", "-o", str(dest)], capture_output=True)
                dest.with_suffix(".json").write_bytes(proc.stdout)
                row["exit"] = proc.returncode
                if proc.returncode == 0:
                    result = Part.read(str(dest))
                    assert result.isValid() and result.isClosed() and len(result.Solids) == 1
                    row["faces"] = len(result.Faces)
                    families = [f.Surface.TypeId for f in result.Faces]
                    if notched:
                        assert "Part::GeomCylinder" not in families
                    elif "Part::GeomCylinder" in families or args.require_analytic:
                        inspect(result, w, length, radius)
                elif args.require_analytic and not notched:
                    raise ValueError(proc.stdout.decode())
            print(json.dumps(row), flush=True)


if __name__ == "__main__":
    main()
