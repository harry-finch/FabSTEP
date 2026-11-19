"""Two disjoint exterior variable fillets on skew edges of a box."""

import argparse
import json
import math
from pathlib import Path
import runpy
import subprocess

import FreeCAD as App
import MeshPart
import Part

V = App.Vector
corner = runpy.run_path(str(Path(__file__).with_name("qualify-two-linear-blends-freecad.py")))["corner"]
CASES = {"vertical": ((2., .15), (4., -.125)), "swapped": ((3.5, -.2), (1.5, .1875))}


def draw(laws, notch):
    box = Part.makeBox(20, 16, 10)
    for i, (base, slope) in enumerate(laws):
        tool = corner(base, slope, [10,16][i])
        if i:
            tool.rotate(V(0,0,0), V(0,1,1), 180)
            tool.translate(V(20,0,0))
        box = box.cut(tool)
    if notch:
        box = box.cut(Part.makeBox(2, 1, 2, V(18,9,-.5)))
    return box.removeSplitter()


def expected_volume(laws):
    # Polygonal integration of the trimmed ellipse in each end and middle
    # section. The removed area is quadratic in the rolling radius.
    volume = 20 * 16 * 10.
    for (radius,slope),length in zip(laws,[10,16]):
        q=1-slope*slope
        theta=math.asin(math.sqrt(q/2))
        def removed(z):
            r=radius+slope*z
            major=r*math.sqrt(1+slope*slope)/q
            minor=r/math.sqrt(q)
            points=[]
            for i in range(1025):
                t=theta*(2*i/1024-1)
                mid=r/q-major*math.cos(t)/math.sqrt(2)
                diag=minor*math.sin(t)/math.sqrt(2)
                points.append((mid+diag,mid-diag))
            return abs(sum(x*v-y*u for (x,y),(u,v) in zip(points,points[1:])))/2
        volume -= length/6*(removed(0)+4*removed(length/2)+removed(length))
    return volume


def inspect(shape, laws, reference):
    assert shape.isValid() and shape.isClosed() and len(shape.Solids)==1
    cones=[f for f in shape.Faces if isinstance(f.Surface,Part.Cone)]
    assert len(cones)==2 and len(shape.Faces)==8
    bounds=[]
    for f in cones:
        ends=[e for e in f.Edges if isinstance(e.Curve,Part.Ellipse)]
        rails=[e for e in f.Edges if not isinstance(e.Curve,Part.Ellipse)]
        for e in rails:
            chord=e.Vertexes[1].Point-e.Vertexes[0].Point
            assert abs(e.Length-chord.Length)<2e-5
            if isinstance(e.Curve,Part.BSplineCurve):
                assert all((v-e.Vertexes[0].Point).cross(chord).Length/chord.Length<2e-5 for v in e.Curve.getPoles())
            else:
                assert isinstance(e.Curve,Part.Line)
        assert len(ends)==2 and len(rails)==2
        for e in ends:
            owners=shape.ancestorsOfType(e,Part.Face)
            assert len(owners)==2 and sum(isinstance(g.Surface,Part.Plane) for g in owners)==1
            cr,a,b=f.curveOnSurface(e)
            for i in range(65):
                t=a+(b-a)*i/64
                uv=cr.value(t)
                assert (f.valueAt(uv.x,uv.y)-e.valueAt(t)).Length<2e-5
        axis=ends[0].Curve.Axis
        assert abs(axis.dot(ends[1].Curve.Axis))>1-1e-8
        along_z=abs(axis.z)>1-1e-8
        radius,slope=laws[0 if along_z else 1]
        assert abs(abs(f.Surface.SemiAngle)-math.asin(abs(slope)/math.sqrt(1+2*slope*slope)))<1e-6
        for end in ends:
            coord=end.Curve.Center.z if along_z else end.Curve.Center.y
            setback=radius+slope*coord
            assert abs(end.Curve.MinorRadius-setback/math.sqrt(1-slope*slope))<2e-5
        bounds.append(axis)
    assert abs(bounds[0].dot(bounds[1]))<1e-8
    assert abs(expected_volume(laws)-shape.Volume)<0.0002
    assert abs(shape.Volume-reference.Volume)<0.0001
    for s in [.1,.35,.65,.9]:
        for d in [.1,.7,1.3]:
            for p in [V(d,d,10*s), V(20-d,16*s,d)]:
                assert shape.isInside(p,1e-7,False)==reference.isInside(p,1e-7,False)


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("output",type=Path)
    p.add_argument("--cli",type=Path)
    p.add_argument("--require-analytic",action="store_true")
    args=p.parse_args()
    args.output.mkdir(exist_ok=True,parents=True)
    for name,laws in {**CASES,"notched":CASES["vertical"]}.items():
        shape=draw(laws,name=="notched")
        ref=args.output/f"{name}-design.step"
        shape.exportStep(str(ref))
        drawing=Part.read(str(ref))
        if name!="notched":
            inspect(drawing,laws,shape)
        for label,angle in [("coarse",.18),("fine",.07)]:
            mesh=MeshPart.meshFromShape(Shape=drawing,LinearDeflection=.02,AngularDeflection=angle,Relative=False)
            src=args.output/f"{name}-{label}.stl"
            mesh.write(str(src))
            row=dict(name=name,label=label,triangles=mesh.CountFacets,reference_mm3=round(drawing.Volume,6))
            if args.cli:
                target=src.with_suffix(".step")
                proc=subprocess.run([str(args.cli.resolve()),"convert",str(src),"--mode","auto","--unit","mm","--force","--json","-o",str(target)],capture_output=True)
                target.with_suffix(".json").write_bytes(proc.stdout)
                row["exit"]=proc.returncode
                if proc.returncode:
                    if args.require_analytic and name!="notched":
                        raise ValueError(proc.stdout.decode())
                else:
                    result=Part.read(str(target))
                    assert result.isClosed() and result.isValid() and len(result.Solids)==1
                    row["faces"]=len(result.Faces)
                    row["cones"]=sum(isinstance(f.Surface,Part.Cone) for f in result.Faces)
                    if name=="notched":
                        assert row["cones"]==0
                    elif row["cones"] or args.require_analytic:
                        inspect(result,laws,drawing)
            print(json.dumps(row),flush=True)


if __name__=="__main__":
    main()
