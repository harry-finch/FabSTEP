"""Independent C1a analytical annuli, sampled without CAD or recognizer output.
Each run contains source rings at 1/3 and 2/3; volume integrates frusta minus bore.
"""
import hashlib
import json
import math
import struct
from pathlib import Path

import generate_m2b
import generate_m3
from generate_m2b import Mesh, transformed

ROOT = Path(__file__).resolve().parent / 'post_v1_chains'
PROFILE = [(0.,12.), (3.,15.), (9.,15.), (12.,11.), (18.,11.), (21.,14.)]


def build(profile):
    mesh = Mesh()
    samples = []
    for (z0,r0),(z1,r1) in zip(profile,profile[1:]):
        samples.extend((z0+(z1-z0)*t/3,r0+(r1-r0)*t/3) for t in range(3))
    samples.append(profile[-1])
    rings = [mesh.ring([(r*math.cos(math.tau*i/96),r*math.sin(math.tau*i/96)) for i in range(96)],z) for z,r in samples]
    for a,b in zip(rings,rings[1:]):
        mesh.wall(a,b)
    bore = [mesh.ring([(5*math.cos(math.tau*i/96),5*math.sin(math.tau*i/96)) for i in range(96)],z) for z in (profile[0][0],profile[-1][0])]
    mesh.wall(*bore,reverse=True)
    mesh.annulus(rings[0],bore[0],True)
    mesh.annulus(rings[-1],bore[1])
    return mesh


def reference(profile, rotate=False, mirror=False):
    center = transformed((0.,0.,0.)) if rotate else (0.,0.,0.)
    axis = tuple(b-a for a,b in zip(center,transformed((0.,0.,1.)))) if rotate else (0.,0.,-1. if mirror else 1.)
    bounds = [[op(c+a*z+sign*r*math.sqrt(max(0.,1-a*a)) for z,r in profile) for c,a in zip(center,axis)] for sign,op in [(-1.,min),(1.,max)]]
    cones = sum(r1!=r0 for (_,r0),(_,r1) in zip(profile,profile[1:]))
    cylinders = len(profile)-cones # includes bore
    volume = math.pi*sum((z1-z0)*((r0*r0+r0*r1+r1*r1)/3-25) for (z0,r0),(z1,r1) in zip(profile,profile[1:]))
    return dict(profile=profile, cones=cones,cylinders=cylinders,planes=2,faces=cones+cylinders+2,volume_mm3=volume,bounds_mm=bounds)


def main():
    ROOT.mkdir(exist_ok=True)
    generate_m2b.ROOT = generate_m3.ROOT = ROOT
    cases = {}
    for name,profile,rotate,mirror in [
        ('three',PROFILE,False,False),
        ('rotated',PROFILE,True,False),
        ('mirrored',PROFILE,False,True),
        ('four',PROFILE+[(27.,14.),(30.,10.)],False,False),
        ('terminal_lands',[(-3.,12.)]+PROFILE+[(24.,14.)],False,False),
    ]:
        mesh=build(profile)
        if mirror:
            mesh.vertices=[(x,y,-z) for x,y,z in mesh.vertices]
            mesh.faces=[(c,b,a) for a,b,c in mesh.faces]
        mesh.write(name,rotate)
        cases[name]=reference(profile,rotate,mirror)
        if name=='three':
            with (ROOT/'binary.stl').open('wb') as f:
                f.write(bytes(80));f.write(struct.pack('<I',len(mesh.faces)))
                for face in mesh.faces:
                    f.write(struct.pack('<12fH',0.,0.,0.,*(c for v in face for c in mesh.vertices[v]),0))
            cases['binary']=dict(cases[name],binary_f32=True)
            obj=generate_m3.mesh_object(1,mesh.vertices,mesh.faces,'Three chamfers')
            generate_m3.write('instances',generate_m3.model([obj],[(1,None),(1,'1 0 0 0 1 0 0 0 -1 40 0 21')]))
            cases['instances']=dict(format='3mf',bodies=2,volume_mm3=2*cases[name]['volume_mm3'],body_bounds_mm=[cases[name]['bounds_mm'],[[25.,-15.,0.],[55.,15.,21.]]])
    for name,case in cases.items():
        path=ROOT/(name+'.'+case.get('format','stl'))
        case['sha256']=hashlib.sha256(path.read_bytes()).hexdigest()
    (ROOT/'manifest.json').write_text(json.dumps(dict(unit='mm',construction='Independent axial frusta and cylinders; bore R5; complete source rings at thirds',cases=cases),indent=2)+'\n')


if __name__=='__main__':
    main()
