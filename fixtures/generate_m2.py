"""Corpus analytique M2 indépendant du moteur FabSTEP (stdlib Python)."""
import json
import math
from pathlib import Path

HERE = Path(__file__).resolve().parent
N = 64


def rotate(p):
    # Rotation propre autour de (1, 2, 3), puis translation non alignée.
    x, y, z = p
    a = (1 / math.sqrt(14), 2 / math.sqrt(14), 3 / math.sqrt(14))
    c, s = math.cos(0.63), math.sin(0.63)
    dot = a[0]*x + a[1]*y + a[2]*z
    cross = (a[1]*z-a[2]*y, a[2]*x-a[0]*z, a[0]*y-a[1]*x)
    return tuple(c*v + s*w + (1-c)*dot*u + t for v,w,u,t in zip(p,cross,a,(13.,-7.,11.)))


def stl(name, vertices, faces, transformed=False):
    if transformed:
        vertices = [rotate(p) for p in vertices]
    lines = ['solid ' + name]
    for face in faces:
        lines += [' facet normal 0 0 0', '  outer loop']
        lines += ['   vertex ' + ' '.join(f'{v:.15g}' for v in vertices[i]) for i in face]
        lines += ['  endloop', ' endfacet']
    lines.append('endsolid ' + name)
    (HERE / (name + '.stl')).write_text('\n'.join(lines) + '\n')


def annular(name, levels, inner=None, transformed=False):
    """N points par anneau; orientation sortante, y compris paroi interne."""
    angles = [2*math.pi*i/N for i in range(N)]
    vertices = []
    faces = []
    def ring(radius, z):
        ids = []
        for a in angles:
            ids.append(len(vertices)); vertices.append((radius*math.cos(a), radius*math.sin(a), z))
        return ids
    rings = [ring(r,z) for z,r in levels]
    def wall(a,b,reverse=False):
        for i in range(N):
            j = (i+1)%N
            tris = [(a[i],a[j],b[j]),(a[i],b[j],b[i])]
            faces.extend([tuple(reversed(t)) for t in tris] if reverse else tris)
    for a,b in zip(rings,rings[1:]): wall(a,b)
    if inner is None:
        for idx, reverse in [(0,True),(-1,False)]:
            center = len(vertices); vertices.append((0,0,levels[idx][0]))
            for i in range(N):
                t=(center,rings[idx][i],rings[idx][(i+1)%N]); faces.append(tuple(reversed(t)) if reverse else t)
    else:
        lower, upper = ring(inner,levels[0][0]), ring(inner,levels[-1][0]); wall(lower,upper,True)
        for outer, inside, reverse in [(rings[0],lower,True),(rings[-1],upper,False)]:
            for i in range(N):
                j=(i+1)%N
                tris=[(inside[i],outer[i],outer[j]),(inside[i],outer[j],inside[j])]
                faces.extend([tuple(reversed(t)) for t in tris] if reverse else tris)
    stl(name,vertices,faces,transformed)


def plate():
    # Seize échantillons par côté, angles définis par les rayons depuis le centre.
    outer = []
    for i in range(16): outer.append((15, -10 + 20*i/16))
    for i in range(16): outer.append((15 - 30*i/16, 10))
    for i in range(16): outer.append((-15, 10 - 20*i/16))
    for i in range(16): outer.append((-15 + 30*i/16, -10))
    angles=[math.atan2(y,x) for x,y in outer]
    vertices=[]
    for z in (0,5):
        vertices += [(x,y,z) for x,y in outer]
        vertices += [(3*math.cos(a),3*math.sin(a),z) for a in angles]
    faces=[]
    for i in range(N):
        j=(i+1)%N; o,i0,ot,it=i,64+i,128+i,192+i; oj,ij,otj,itj=j,64+j,128+j,192+j
        faces += [(o,oj,otj),(o,otj,ot), (i0,itj,ij),(i0,it,itj), (it,ot,otj),(it,otj,itj), (i0,oj,o),(i0,ij,oj)]
    stl('m2_plate_hole',vertices,faces)
    gaps=[(angles[(i+1)%N]-angles[i])%(2*math.pi) for i in range(N)]
    return 3*(1-math.cos(max(gaps)/2))


def main():
    annular('m2_cylinder',[(0,10),(20,10)])
    annular('m2_cylinder_subdivided',[(0,10),(5,10),(10,10),(15,10),(20,10)])
    annular('m2_cylinder_rotated',[(0,10),(5,10),(10,10),(15,10),(20,10)],transformed=True)
    annular('m2_tube',[(0,10),(20,10)],inner=5)
    annular('m2_conical_frustum',[(0,10),(20,8)])
    annular('m2_waisted',[(0,10),(5,10),(10,8),(15,10),(20,10)])
    plate_error=plate(); chord10=10*(1-math.cos(math.pi/N))
    cases={
      'm2_cylinder':dict(analytic_faces=3,planes=2,cylinders=1,radii_mm=[10],volume_mm3=math.pi*10**2*20,max_chord_error_mm=chord10),
      'm2_cylinder_subdivided':dict(analytic_faces=3,planes=2,cylinders=1,radii_mm=[10],volume_mm3=math.pi*10**2*20,max_chord_error_mm=chord10),
      'm2_cylinder_rotated':dict(analytic_faces=3,planes=2,cylinders=1,radii_mm=[10],volume_mm3=math.pi*10**2*20,max_chord_error_mm=chord10,axis=[rotate((0,0,1))[i]-rotate((0,0,0))[i] for i in range(3)]),
      'm2_tube':dict(analytic_faces=4,planes=2,cylinders=2,radii_mm=[5,10],volume_mm3=math.pi*(10**2-5**2)*20,max_chord_error_mm=chord10),
      'm2_plate_hole':dict(analytic_faces=7,planes=6,cylinders=1,radii_mm=[3],volume_mm3=(600-9*math.pi)*5,max_chord_error_mm=plate_error),
      'm2_conical_frustum':dict(negative='tronc de cône ; ne pas reconnaître comme cylindre droit',radii_mm=[8,10],volume_mm3=math.pi*20*(100+80+64)/3,max_chord_error_mm=chord10),
      'm2_waisted':dict(negative='constriction intermédiaire ; ne pas reconnaître comme cylindre droit',radii_mm=[8,10],volume_mm3=math.pi*(1000+2*5*(100+80+64)/3),max_chord_error_mm=chord10),
    }
    (HERE/'m2_manifest.json').write_text(json.dumps({'unit':'mm','circumferential_segments':N,'cases':cases},ensure_ascii=False,indent=2)+'\n')

if __name__ == '__main__': main()
