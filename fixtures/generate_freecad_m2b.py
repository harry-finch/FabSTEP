"""Fixture indépendante produite par FreeCAD Part, sans code FabSTEP.

Exécuter avec le Python de FreeCAD et son dossier lib dans PYTHONPATH.
La tessellation reste exportée en ASCII pour conserver la précision des points.
"""
import json
import math
from pathlib import Path
import FreeCAD as App
import Part

here = Path(__file__).resolve().parent
box = Part.makeBox(30, 20, 10, App.Vector(-15, -10, 0))
vertical = Part.makeCylinder(3, 6, App.Vector(-6, 0, 10), App.Vector(0, 0, -1))
horizontal = Part.makeCylinder(2, 5, App.Vector(15, 0, 5), App.Vector(-1, 0, 0))
shape = box.cut(vertical).cut(horizontal)
assert shape.isValid() and shape.isClosed() and len(shape.Solids) == 1
points, faces = shape.tessellate(0.005)
name = 'm2b_orthogonal_freecad'
lines = ['solid ' + name]
for face in faces:
    lines += [' facet normal 0 0 0', '  outer loop']
    for i in face:
        p = points[i]
        lines.append(f'   vertex {p.x:.17g} {p.y:.17g} {p.z:.17g}')
    lines += ['  endloop', ' endfacet']
lines.append('endsolid ' + name)
(here / (name + '.stl')).write_text('\n'.join(lines) + '\n')
report = dict(generator='FreeCAD Part', version='.'.join(App.Version()[:3]),
              case=name, unit='mm', faces=10, planes=8, cylinders=2,
              radii_mm=[2, 3], depths_mm=[5, 6], axes=[[1,0,0],[0,0,1]],
              volume_mm3=6000-74*math.pi, tessellation_deflection_mm=0.005,
              source_triangles=len(faces), nominal_volume_check=shape.Volume)
(here / (name + '.json')).write_text(json.dumps(report, indent=2) + '\n')
print(name, len(faces), shape.Volume)
