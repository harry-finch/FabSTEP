"""Independent C1b annuli with resolved lands, fixed before recognition.

Reuse only the analytical source mesher/reference formulas from C1a. No CAD or
recognizer output participates. Every nominal run has two interior source rings.
"""
import hashlib
import json
import struct
from pathlib import Path

import generate_m2b
import generate_m3
from generate_post_v1_chains import build, reference

ROOT = Path(__file__).resolve().parent / 'same_sign_chains'
TWO = [(0., 10.), (3., 13.), (9., 13.), (12., 16.)]
DOWN = [(0., 22.), (3., 19.), (9., 19.), (12., 16.), (18., 16.), (21., 13.)]
MIXED = [(0., 12.), (3., 15.), (9., 15.), (12., 18.), (18., 18.), (21., 14.)]


def main():
    ROOT.mkdir(exist_ok=True)
    generate_m2b.ROOT = generate_m3.ROOT = ROOT
    cases = {}
    negative_cases = {}
    for name, profile, rotate in [
        ('two_increasing', TWO, False),
        ('two_rotated', TWO, True),
        ('three_decreasing', DOWN, False),
        ('mixed_signs', MIXED, False),
        # C1b2 intentionally promotes the existing, witnessed no-land source.
        ('no_land', [(0., 10.), (3., 13.), (9., 16.)], False),
    ]:
        mesh = build(profile)
        mesh.write(name, rotate)
        cases[name] = reference(profile, rotate)
        if name == 'two_increasing':
            with (ROOT / 'binary.stl').open('wb') as stream:
                stream.write(bytes(80))
                stream.write(struct.pack('<I', len(mesh.faces)))
                for face in mesh.faces:
                    stream.write(struct.pack('<12fH', 0., 0., 0., *(c for v in face for c in mesh.vertices[v]), 0))
            cases['binary'] = dict(cases[name], binary_f32=True)
            obj = generate_m3.mesh_object(1, mesh.vertices, mesh.faces, 'Two same-sign cones')
            generate_m3.write('instances', generate_m3.model([obj], [(1, None), (1, '1 0 0 0 1 0 0 0 -1 40 0 12')]))
            cases['instances'] = dict(format='3mf', bodies=2, cones=4, cylinders=4,
                                      volume_mm3=2 * cases[name]['volume_mm3'],
                                      body_bounds_mm=[cases[name]['bounds_mm'], [[24., -16., 0.], [56., 16., 12.]]])
    for name, profile in [
        ('curved_meridian', [(z, 10. + .01*z*z) for z in (0., 1., 2., 3., 5., 7., 9., 10., 11., 12.)]),
        ('late_ambiguous_land', [(0., 10.), (3., 13.), (9., 13.), (12., 16.), (12.00015, 16.), (15., 16.5)]),
    ]:
        build(profile).write(name)
        negative_cases[name] = dict(profile=profile, primitives_error='recognition.conic_unproven',
                                    surfaces_error='recognition.surface_unproven', auto_cones=0,
                                    sha256=hashlib.sha256((ROOT / (name + '.stl')).read_bytes()).hexdigest())
    for name, case in cases.items():
        path = ROOT / (name + '.' + case.get('format', 'stl'))
        case['sha256'] = hashlib.sha256(path.read_bytes()).hexdigest()
    (ROOT / 'manifest.json').write_text(json.dumps(dict(unit='mm', construction='Independent analytical frusta and resolved cylinders; R5 bore; source rings at thirds', cases=cases, negative_cases=negative_cases), indent=2) + '\n')


if __name__ == '__main__':
    main()
