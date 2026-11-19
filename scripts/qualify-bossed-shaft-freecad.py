"""Build the independent R-117 longitudinal boss specimens with FreeCAD.

Run with the FreeCAD Python environment and a temporary output directory.
An optional second argument names a FabSTEP CLI to check the converted STEP files.
"""
import argparse
import math
from pathlib import Path
import subprocess

import FreeCAD as App
import MeshPart
import Part


def qualify(output, cli):
    output.mkdir(parents=True, exist_ok=True)
    height = 3.5
    radius = 5.
    boss = Part.makeBox(8., 6., height, App.Vector(0, -3, 0))
    source = Part.makeCylinder(radius, height).fuse(boss).removeSplitter()
    assert source.isValid() and len(source.Faces) == 6 and len(source.Solids) == 1
    angle = 2 * math.atan2(3., 4.)
    volume = (math.pi*radius**2 - radius**2/2*(angle - math.sin(angle)) + 24.)*height
    assert abs(source.Volume - volume) < 1e-8
    for name, linear, angular in [('coarse', .02, .25), ('fine', .006, .14)]:
        mesh = MeshPart.meshFromShape(Shape=source, LinearDeflection=linear,
                                      AngularDeflection=angular, Relative=False)
        stl = output / f'r117-bossed-shaft-{name}.stl'
        mesh.write(str(stl))
        if cli:
            step = stl.with_suffix('.step')
            subprocess.run([cli, 'convert', stl, '--mode', 'auto', '-o', step, '--force'], check=True)
            solid = Part.read(str(step))
            assert solid.isValid() and len(solid.Solids) == 1 and len(solid.Faces) == 6
            assert abs(solid.Volume-volume) < 2e-5
            [wall] = [f for f in solid.Faces if f.Surface.TypeId == 'Part::GeomCylinder']
            start, end, _, _ = wall.ParameterRange
            assert abs(wall.Surface.Radius-radius) < 1e-6 and abs(end-start-(math.tau-angle)) < 1e-6
            assert len(wall.Edges) == 4
            print(f'{name}: {mesh.CountFacets} triangles, {solid.Volume:.9f} mm³, {end-start:.9f} rad')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('cli', nargs='?', type=Path)
    args = parser.parse_args()
    qualify(args.output, args.cli)
