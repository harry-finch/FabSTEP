"""Independent analytical zone corpus: sphere R10 intersected by two source planes.
No recognizer/native output is used to generate geometry or expected measurements.
"""
import hashlib
import json
import math
import struct
from pathlib import Path

import generate_m2b
import generate_m3
from generate_m2b import Mesh, boundary, transformed

ROOT = Path(__file__).resolve().parent / 'post_v1_zones'
R = 10.


def build(low=-4., high=6., count=80, rows=16, concave=False, irregular=False):
    mesh = Mesh()
    outer = boundary(-15., 15., -15., 15., count // 4) if concave else None
    angles = [math.atan2(y, x) for x, y in outer] if concave else [math.tau * i / count for i in range(count)]
    rings = []
    for j in range(rows + 1):
        v = math.asin(low / R) + (math.asin(high / R) - math.asin(low / R)) * j / rows
        points = []
        for u in angles:
            local_v = v + (0.006 * math.sin(3 * u + j) if irregular and j not in (0, rows) else 0.)
            points.append(mesh.point((R * math.cos(local_v) * math.cos(u), R * math.cos(local_v) * math.sin(u), R * math.sin(local_v))))
        rings.append(points)
    for a, b in zip(rings, rings[1:]):
        mesh.wall(a, b, concave)
    if concave:
        bottom, top = mesh.ring(outer, low), mesh.ring(outer, high)
        mesh.wall(bottom, top)
        mesh.annulus(bottom, rings[0], True)
        mesh.annulus(top, rings[-1])
    else:
        mesh.fan(rings[0], (0., 0., low), True)
        mesh.fan(rings[-1], (0., 0., high))
    return mesh


def reference(low, high, concave=False, rotated=False):
    volume = math.pi * (R * R * (high - low) - (high**3 - low**3) / 3.)
    center = transformed((0., 0., 0.)) if rotated else (0., 0., 0.)
    axis = tuple(b-a for a,b in zip(center, transformed((0.,0.,1.)))) if rotated else (0.,0.,1.)
    lo, hi = math.asin(low/R), math.asin(high/R)
    bounds = []
    for sign in (-1., 1.):
        corner = []
        for c, a in zip(center, axis):
            v = min(hi, max(lo, math.atan2(sign*a, math.sqrt(max(0., 1.-a*a)))))
            corner.append(c + sign*R*(sign*a*math.sin(v) + math.sqrt(max(0., 1.-a*a))*math.cos(v)))
        bounds.append(corner)
    if concave:
        volume = 900. * (high-low) - volume
        bounds = [[-15.,-15.,low],[15.,15.,high]]
    return dict(radius_mm=R, center_mm=center, axis=axis, latitudes_deg=[math.degrees(lo),math.degrees(hi)],
                faces=7 if concave else 3, planes=6 if concave else 2, spheres=1,
                volume_mm3=volume, bounds_mm=bounds, inner=concave)


def binary(mesh, name):
    with (ROOT / f'{name}.stl').open('wb') as f:
        f.write(bytes(80)); f.write(struct.pack('<I', len(mesh.faces)))
        for face in mesh.faces:
            f.write(struct.pack('<12fH', 0.,0.,0., *(c for v in face for c in mesh.vertices[v]), 0))


def main():
    ROOT.mkdir(exist_ok=True)
    generate_m2b.ROOT = generate_m3.ROOT = ROOT
    cases = {}
    for name, low, high, kwargs, rotate in [
        ('cross_equator',-4.,6.,{},False),
        ('above_equator',2.,8.,{},False),
        ('below_equator',-8.,-2.,{},False),
        ('rotated',-4.,6.,{},True),
        ('irregular',-4.,6.,dict(irregular=True),False),
        ('dense',-4.,6.,dict(count=128,rows=24),False),
        ('concave',-4.,6.,dict(concave=True),False),
    ]:
        mesh = build(low, high, **kwargs)
        mesh.write(name, rotate)
        cases[name] = dict(reference(low, high, kwargs.get('concave',False), rotate), triangles=len(mesh.faces))
        if name == 'cross_equator':
            binary(mesh, 'binary')
            cases['binary'] = dict(cases[name], binary_f32=True)
            resource = generate_m3.mesh_object(1,mesh.vertices,mesh.faces,'Zone')
            generate_m3.write('instances', generate_m3.model([resource],[(1,None),(1,'1 0 0 0 1 0 0 0 -1 35 0 2')]))
            cases['instances'] = dict(format='3mf', bodies=2, volume_mm3=2*cases[name]['volume_mm3'],
                body_bounds_mm=[cases[name]['bounds_mm'], [[25.,-10.,-4.],[45.,10.,6.]]])
            generate_m3.write('touching', generate_m3.model([resource],[(1,None),(1,'1 0 0 0 1 0 0 0 1 20 0 0')]))
            cases['touching'] = dict(format='3mf', expected_refusal='cad.bodies_not_separated')
    for name, mesh, reason in [
        ('sparse', build(count=12, rows=3), 'insufficient deviation budget'),
        ('spheroid', build(), 'non-spherical meridian'),
        ('missing_triangle', build(), 'source boundary'),
        ('folded', build(), 'folded spherical projection'),
    ]:
        if name == 'spheroid':
            mesh.vertices = [(x,y,z*1.15) for x,y,z in mesh.vertices]
        elif name == 'missing_triangle':
            mesh.faces.pop(10)
        elif name == 'folded':
            mesh.vertices[80*7+10], mesh.vertices[80*9+10] = mesh.vertices[80*9+10], mesh.vertices[80*7+10]
        mesh.write(name)
        cases[name] = dict(expected_refusal=reason, triangles=len(mesh.faces))
    for name, case in cases.items():
        path = ROOT / (name + '.' + case.get('format', 'stl'))
        case['sha256'] = hashlib.sha256(path.read_bytes()).hexdigest()
    (ROOT/'manifest.json').write_text(json.dumps(dict(unit='mm',construction='R10 spherical zones; actual source disks or perforated block planes',cases=cases),indent=2)+'\n')


if __name__ == '__main__':
    main()
