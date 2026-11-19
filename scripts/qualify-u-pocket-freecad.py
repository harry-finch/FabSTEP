"""Design and independently read back a rectangular pocket with three or four coupled bottom fillets."""

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


def drawing(w, length, r, notched, closed=False):
    blank = Part.makeBox(44, 36, FLOOR + DEPTH)
    cutter = Part.makeBox(w, length, DEPTH + 1, V(5, 6, FLOOR))
    edges = [e for e in cutter.Edges if abs(e.CenterOfMass.z - FLOOR) < 1e-8
             and (closed or abs(e.CenterOfMass.y - (6 + length)) > 1e-8)]
    assert len(edges) == (4 if closed else 3)
    cutter = cutter.makeFillet(r, edges)
    solid = blank.cut(cutter).removeSplitter()
    if notched:
        # Break only the left junction. The cylinder network then has
        # extra planar boundaries and cannot satisfy its incidence certificate.
        solid = solid.cut(Part.makeBox(r, r, DEPTH + 2, V(5 + r / 4, 6 + r / 4, FLOOR - 1))).removeSplitter()
        assert len(solid.Faces) > 14
    return solid


def inspect(solid, w, length, radius, closed=False):
    assert solid.isValid() and solid.isClosed() and len(solid.Solids) == 1
    cylinders = [f for f in solid.Faces if isinstance(f.Surface, Part.Cylinder)]
    seams = [e for e in solid.Edges if isinstance(e.Curve, Part.Ellipse)]
    walls = 4 if closed else 3
    assert len(cylinders) == walls and len(solid.Faces) == 11 + walls and len(seams) == (4 if closed else 2)
    if closed:
        floors=[f for f in solid.Faces if isinstance(f.Surface,Part.Plane)
                and all(abs(v.Point.z-FLOOR)<2e-5 for v in f.Vertexes)]
        (floor,)=floors
        assert abs(floor.Area-(w-2*radius)*(length-2*radius))<2e-5
        assert len(floor.Edges)==4 and all(isinstance(e.Curve,Part.Line) for e in floor.Edges)
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
    assert sorted(degree) == ([2]*4 if closed else [1,1,2])
    circles = [e for e in solid.Edges if isinstance(e.Curve, Part.Circle)]
    assert len(circles) == (0 if closed else 2)
    # Integrate the rectangular cavity width and length over its lower quarter circle.
    volume = 44 * 36 * (DEPTH + FLOOR) - w * length * DEPTH + (2 * length + (2 if closed else 1)*w) * (
        1 - math.pi / 4) * radius**2 - (4 if closed else 2) * (5/3 - math.pi/2) * radius**3
    assert abs(solid.Volume - volume) < 0.001, (solid.Volume, volume)
    for x, y in [(0.4, 0.4), (w - 0.6, 0.6), (w / 2, 0.4), (0.4, length - 0.2), (w-0.4, length-0.4)]:
        d = min(x, w - x, y, length-y if closed else radius)
        z = FLOOR + radius - math.sqrt(radius**2 - (radius - d)**2)
        for off in [-0.04, 0.04]:
            assert solid.isInside(V(5+x, 6+y, z+off), 1e-7, False) == (off < 0)
    return round(volume, 6)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("directory", type=Path)
    p.add_argument("--cli", type=Path)
    p.add_argument("--require-analytic", action="store_true")
    p.add_argument("--closed",action="store_true",help="Round all four bottom edges")
    args = p.parse_args()
    args.directory.mkdir(exist_ok=True, parents=True)
    cases = {**CASES, "notched": CASES["u"]}
    if args.closed:
        cases["through"] = CASES["u"]
    for name, (w, length, radius) in cases.items():
        notched = name == "notched"
        if args.closed and name=="u":
            name="loop"
        ref = args.directory / f"{name}-design.step"
        source = drawing(w, length, radius, notched, args.closed)
        if name == "through":
            # Retain the four cylinders and elliptic corners while removing
            # the entire rectangular floor. The core must not restore it.
            through = Part.makeBox(w-2*radius, length-2*radius, FLOOR+1,
                                   V(5+radius, 6+radius, -0.5))
            source = source.cut(through).removeSplitter()
            assert source.isClosed() and source.isValid() and len(source.Solids)==1
            assert sum(isinstance(e.Curve,Part.Ellipse) for e in source.Edges)==4
            assert not any(all(abs(v.Point.z-FLOOR)<1e-6 for v in f.Vertexes) for f in source.Faces)
        source.exportStep(str(ref))
        design = Part.read(str(ref))
        negative = notched or name == "through"
        expected = None if negative else inspect(design, w, length, radius,args.closed)
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
                    if negative:
                        assert "Part::GeomCylinder" not in families
                        if name == "through":
                            assert not result.isInside(V(5+w/2,6+length/2,FLOOR/2),1e-7,False)
                    elif "Part::GeomCylinder" in families or args.require_analytic:
                        inspect(result, w, length, radius,args.closed)
                elif args.require_analytic and not negative:
                    raise ValueError(proc.stdout.decode())
            print(json.dumps(row), flush=True)


if __name__ == "__main__":
    main()
