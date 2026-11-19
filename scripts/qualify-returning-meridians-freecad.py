"""Draw and independently verify two turned parts with a genuine axial undercut."""

import argparse
import math
from pathlib import Path
import FreeCAD as App
import MeshPart
import Part

V = App.Vector
# Inner and outer cap radii, and the four axial poles of a cubic whose radius
# is affine. Both meridians travel upwards, downwards and upwards again.
CASES = {
    "flared": (3.0, 10.5, (0.0, 13.0, -5.0, 8.0)),
    "inverted": (11.0, 4.0, (0.0, 15.0, -3.0, 10.0)),
}


def height(z, t):
    return sum(math.comb(3, j) * (1-t)**(3-j) * t**j * h for j,h in enumerate(z))


def profile_point(r0, r1, z, t):
    return V(r0 + (r1-r0)*t, 0, height(z,t))


def draw(r0, r1, z):
    c = Part.BezierCurve()
    c.setPoles([V(r0+(r1-r0)*i/3,0,h) for i,h in enumerate(z)])
    a,b = V(0,0,0),V(0,0,z[3])
    edges = [Part.makeLine(a,c.value(0)),c.toShape(),
             Part.makeLine(c.value(1),b),Part.makeLine(b,a)]
    return Part.Face(Part.Wire(edges)).revolve(a,V(0,0,1),360)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--mesh-dir", type=Path)
    parser.add_argument("--check", action="store_true")
    args=parser.parse_args()
    args.output.mkdir(exist_ok=True,parents=True)
    if args.mesh_dir: args.mesh_dir.mkdir(exist_ok=True,parents=True)
    for name,(r0,r1,z) in CASES.items():
        drawing=draw(r0,r1,z)
        if name=="inverted":
            drawing.rotate(V(),V(1,0,0),90)
            drawing.translate(V(3,-5,2))
        drawing.exportStep(str(args.output / f"{name}-reference.step"))
        # Verify two axial extrema and a physically resolvable undercut.
        pts=[height(z,i/1000) for i in range(1001)]
        fall=max(max(pts[:i+1])-p for i,p in enumerate(pts))
        assert fall>1 and min(pts)==0 and max(pts)==z[-1]
        assert len(drawing.Solids)==1 and drawing.Solids[0].isClosed() and drawing.isValid()
        for variant,linear,angle in [("coarse",.009,.18),("fine",.007,.13)]:
            key=f"{name}-{variant}"
            if args.mesh_dir:
                reimport=Part.read(str(args.output / f"{name}-reference.step"))
                mesh=MeshPart.meshFromShape(Shape=reimport,LinearDeflection=linear,
                                            AngularDeflection=angle,Relative=False)
                mesh.write(str(args.mesh_dir/f"{key}.stl"))
                print(key,mesh.CountFacets,"axial return",fall,"volume",drawing.Volume)
            if args.check:
                solid=Part.read(str(args.output / f"{key}.step"))
                assert solid.isValid() and len(solid.Solids)==1 and solid.Solids[0].isClosed()
                assert len(solid.Faces)==3
                (wall,)=[f for f in solid.Faces if isinstance(f.Surface,Part.SurfaceOfRevolution)]
                (reference,)=[f for f in drawing.Faces if isinstance(f.Surface,Part.SurfaceOfRevolution)]
                u0,u1,v0,v1=reference.ParameterRange
                error=max(wall.distToShape(Part.Vertex(reference.valueAt(
                    u0+i*(u1-u0)/5,v0+j*(v1-v0)/48)))[0] for i in range(5) for j in range(49))
                assert error<3e-5 and abs(solid.Volume-drawing.Volume)<2e-5*drawing.Volume
                # Two material probes straddle the wall away from the axial
                # extrema. Repeat inside the undercut, at three azimuths.
                for t in (.17,.31,.5,.69,.83):
                    r=r0+(r1-r0)*t
                    for sign in (-1,1):
                        z_probe=height(z,t)+sign*.04
                        for angle in (.12,2.52,4.79):
                            x,y=r*math.cos(angle),r*math.sin(angle)
                            p=(V(x,y,z_probe) if name=="flared" else V(x+3,-z_probe-5,y+2))
                            inside=(r1-r0)*sign>0
                            assert drawing.isInside(p,1e-7,True)==inside
                            assert solid.isInside(p,1e-7,True)==inside
                print(key,"support distance",error,"volume",solid.Volume,"material probes",30)


if __name__=="__main__":
    main()
