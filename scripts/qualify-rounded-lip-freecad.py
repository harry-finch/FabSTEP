"""Draw two open cups with independently rounded inner and outer rims, then reopen FabSTEP exports."""
import argparse
import json
import math
from pathlib import Path
import subprocess

import FreeCAD as App
import MeshPart
import Part


def draw(inner, outer, outer_fillet, inner_fillet):
    body = Part.makeCylinder(outer, 14).cut(Part.makeCylinder(inner, 13, App.Vector(0, 0, 2)))
    for radius, fillet in ((outer, outer_fillet), (inner, inner_fillet)):
        edges = [e for e in body.Edges if isinstance(e.Curve, Part.Circle)
                 and abs(e.Curve.Center.z - 14) < 1e-7 and abs(e.Curve.Radius - radius) < 1e-7]
        assert len(edges) == 1
        body = body.makeFillet(fillet, edges)
    return body.removeSplitter()


def check_material(shape, inner, outer, outer_fillet, inner_fillet):
    for radius, fillet, direction in [(inner, inner_fillet, 1), (outer, outer_fillet, -1)]:
        for t in (0.1, 0.5, 0.9):
            z = 14 - fillet + t * fillet
            wall = radius + direction * fillet * (1 - math.sqrt(1 - t * t))
            for delta in (-0.02, 0.02):
                p = App.Vector(wall + delta, 0, z)
                assert shape.isInside(p, 1e-7, False) == (direction * delta > 0)
    assert shape.isInside(App.Vector(0, 0, 1), 1e-7, False)
    assert not shape.isInside(App.Vector(0, 0, 3), 1e-7, False)


def run(args, name, inner, outer, outer_fillet, inner_fillet):
    drawing = draw(inner, outer, outer_fillet, inner_fillet)
    drawing.exportStep(str(args.output / (name + "-drawing.step")))
    for density, chord, angle in [("coarse", 0.012, 0.16), ("fine", 0.006, 0.10)]:
        mesh = MeshPart.meshFromShape(Shape=drawing, LinearDeflection=chord,
                                      AngularDeflection=angle, Relative=False)
        stl = args.output / f"{name}-{density}.stl"
        step = stl.with_suffix(".step")
        mesh.write(str(stl))
        subprocess.run([args.cli, "convert", str(stl), "--mode", "auto", "-o", str(step), "--force"], check=True)
        reopened = Part.read(str(step))
        assert len(reopened.Solids) == 1 and reopened.Solids[0].isClosed() and reopened.isValid()
        assert len(reopened.Faces) == 7
        assert sum(isinstance(f.Surface, Part.Toroid) for f in reopened.Faces) == 2
        assert sum(isinstance(f.Surface, Part.Cylinder) for f in reopened.Faces) == 2
        assert sum(isinstance(f.Surface, Part.Plane) for f in reopened.Faces) == 3
        assert abs(reopened.Volume - drawing.Volume) < 0.002
        check_material(reopened, inner, outer, outer_fillet, inner_fillet)
        print(json.dumps(dict(case=name, density=density, triangles=mesh.CountFacets,
                              volume_mm3=reopened.Volume)), flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--cli", required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    run(args, "small", 8, 10, 0.45, 0.65)
    run(args, "wide", 13, 15, 0.65, 0.45)
