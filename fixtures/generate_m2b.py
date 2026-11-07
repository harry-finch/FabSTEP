"""Maillages M2b indépendants : surfaces polygonales à référence analytique."""
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parent
H = 10.0


def transformed(p):
    # Axe (1,2,3) et angle arbitraire ; translation non alignée.
    x, y, z = p
    a = (1/math.sqrt(14), 2/math.sqrt(14), 3/math.sqrt(14))
    c, s = math.cos(.63), math.sin(.63)
    dot = sum(u*v for u,v in zip(a,p))
    cross = (a[1]*z-a[2]*y, a[2]*x-a[0]*z, a[0]*y-a[1]*x)
    return tuple(c*v+s*w+(1-c)*dot*u+t for v,w,u,t in zip(p,cross,a,(13.,-7.,11.)))


class Mesh:
    def __init__(self):
        self.vertices, self.faces = [], []
    def point(self, p):
        self.vertices.append(p); return len(self.vertices)-1
    def triangle(self, a, b, c, reverse=False):
        self.faces.append((c,b,a) if reverse else (a,b,c))
    def ring(self, points, z):
        return [self.point((x,y,z)) for x,y in points]
    def annulus(self, outer, inner, reverse=False):
        for i in range(len(outer)):
            j=(i+1)%len(outer)
            self.triangle(inner[i],outer[i],outer[j],reverse)
            self.triangle(inner[i],outer[j],inner[j],reverse)
    def fan(self, ring, center, reverse=False):
        c=self.point(center)
        for i in range(len(ring)):
            self.triangle(c,ring[i],ring[(i+1)%len(ring)],reverse)
    def wall(self, lower, upper, reverse=False):
        for i in range(len(lower)):
            j=(i+1)%len(lower)
            self.triangle(lower[i],lower[j],upper[j],reverse)
            self.triangle(lower[i],upper[j],upper[i],reverse)
    def write(self, name, rotate=False):
        vertices = [transformed(p) for p in self.vertices] if rotate else self.vertices
        lines=['solid '+name]
        for face in self.faces:
            lines += [' facet normal 0 0 0','  outer loop']
            lines += ['   vertex '+' '.join(f'{c:.15g}' for c in vertices[i]) for i in face]
            lines += ['  endloop',' endfacet']
        (ROOT/(name+'.stl')).write_text('\n'.join(lines+['endsolid '+name])+'\n')


def boundary(x0,x1,y0,y1,n):
    # Compte identique sur chaque bord partagé ; coins inclus une fois.
    return ([(x1,y0+(y1-y0)*i/n) for i in range(n)] +
            [(x1-(x1-x0)*i/n,y1) for i in range(n)] +
            [(x0,y1-(y1-y0)*i/n) for i in range(n)] +
            [(x0+(x1-x0)*i/n,y0) for i in range(n)])


def build(name, features, rotate=False, subdivisions=1, n=16):
    # Deux cellules 15×20 pour deux objets, une cellule 30×20 sinon.
    mesh=Mesh()
    cells = [(-15.,0.),(0.,15.)] if len(features)==2 else [(-15.,15.)]
    for (x0,x1), feature in zip(cells,features):
        y0,y1=-10.,10.; cx=(x0+x1)/2; cy=0.
        outer=boundary(x0,x1,y0,y1,n)
        angles=[math.atan2(y-cy,x-cx) for x,y in outer]
        def circle(radius): return [(cx+radius*math.cos(a),cy+radius*math.sin(a)) for a in angles]
        outer0=mesh.ring(outer,0); outerH=mesh.ring(outer,H)
        kind=feature['kind']; radius=feature['radius']; top=mesh.ring(circle(radius),H)
        mesh.annulus(outerH,top)
        if kind=='boss':
            mesh.fan(outer0,(cx,cy,0),True)
            levels=[H+(feature['height']*i/subdivisions) for i in range(subdivisions+1)]
            rings=[top]+[mesh.ring(circle(radius),z) for z in levels[1:]]
            if feature.get('bulge'):
                rings[len(rings)//2]=mesh.ring(circle(radius+1),levels[len(levels)//2])
            for a,b in zip(rings,rings[1:]): mesh.wall(a,b)
            mesh.fan(rings[-1],(cx,cy,levels[-1]))
        else:
            depth=feature['depth']; floor=H-depth
            if kind=='through':
                bottom=mesh.ring(circle(radius),0); mesh.annulus(outer0,bottom,True)
            else:
                mesh.fan(outer0,(cx,cy,0),True)
                bottom=mesh.ring(circle(feature.get('bottom_radius',radius)),floor)
            # Build explicit lower-to-upper wall, optionally subdivided.
            rings=[bottom]
            for k in range(1,subdivisions):
                t=k/subdivisions; r=feature.get('bottom_radius',radius)*(1-t)+radius*t
                rings.append(mesh.ring(circle(r),floor+(H-floor)*t))
            rings.append(top)
            for a,b in zip(rings,rings[1:]): mesh.wall(a,b,True)
            if kind!='through': mesh.fan(bottom,(cx,cy,floor+(1 if feature.get('uneven_floor') else 0)))
        # Outer box walls only; shared boundaries remain top/bottom seams.
        for i,(x,y) in enumerate(outer):
            j=(i+1)%len(outer); xx,yy=outer[j]
            if ((x==xx and x in (-15.,15.)) or (y==yy and y in (-10.,10.))):
                mesh.triangle(outer0[i],outer0[j],outerH[j]); mesh.triangle(outer0[i],outerH[j],outerH[i])
    mesh.write(name,rotate)


def main():
    blind={'kind':'blind','radius':3.,'depth':6.}
    through={'kind':'through','radius':3.,'depth':10.}
    boss={'kind':'boss','radius':3.,'height':5.}
    specs={
      'm2b_blind':([blind],False,1,16),
      'm2b_blind_subdivided':([blind],False,4,16),
      'm2b_blind_rotated':([blind],True,4,16),
      'm2b_two_holes':([blind,through],False,4,16),
      'm2b_boss':([boss],False,4,16),
      'm2b_boss_blind':([boss,blind],False,4,16),
      'm2b_boss_blind_rotated':([boss,blind],True,4,16),
      'm2b_conical_blind':([dict(blind,bottom_radius=2.4)],False,4,16),
      'm2b_nonflat_floor':([dict(blind,uneven_floor=True)],False,4,16),
      'm2b_bulged_boss':([dict(boss,bulge=True)],False,4,16),
      'm2b_low_sample_blind':([blind],False,1,2),
    }
    for name,(features,rotated,subdivisions,n) in specs.items(): build(name,features,rotated,subdivisions,n)
    base=30*20*10; disk=math.pi*3**2
    cases={
      'm2b_blind':dict(faces=8,cylinders=1,radii_mm=[3],depths_mm=[6],volume_mm3=base-disk*6),
      'm2b_blind_subdivided':dict(same_as='m2b_blind',axial_intervals=4),
      'm2b_blind_rotated':dict(same_as='m2b_blind',axial_intervals=4,transformed=True),
      'm2b_two_holes':dict(faces=9,cylinders=2,radii_mm=[3,3],depths_mm=[6,10],volume_mm3=base-disk*16),
      'm2b_boss':dict(faces=8,cylinders=1,radii_mm=[3],heights_mm=[5],volume_mm3=base+disk*5),
      'm2b_boss_blind':dict(faces=10,cylinders=2,radii_mm=[3,3],depths_mm=[6],heights_mm=[5],volume_mm3=base-disk),
      'm2b_boss_blind_rotated':dict(same_as='m2b_boss_blind',transformed=True),
      'm2b_conical_blind':dict(negative='paroi conique, pas un cylindre droit',radii_mm=[2.4,3],depths_mm=[6],volume_mm3=base-math.pi*6*(3**2+3*2.4+2.4**2)/3),
      'm2b_nonflat_floor':dict(negative='fond conique, pas un fond plan',radii_mm=[3],depths_mm=[6],volume_mm3=base-disk*(6-1/3)),
      'm2b_bulged_boss':dict(negative='saillie intermédiaire, pas un cylindre droit',radii_mm=[3,4],heights_mm=[5],volume_mm3=base+math.pi*1.25*(18+2*37/3)),
      'm2b_low_sample_blind':dict(negative='cercle échantillonné seulement sur 8 sommets ; intention ambiguë',radii_mm=[3],depths_mm=[6],vertices_per_ring=8,volume_mm3=base-disk*6),
    }
    for name,(features,rotated,_,n) in specs.items():
        widths=[(-15.,0.),(0.,15.)] if len(features)==2 else [(-15.,15.)]
        gaps=[]
        for x0,x1 in widths:
            cx=(x0+x1)/2
            angles=[math.atan2(y,x-cx) for x,y in boundary(x0,x1,-10,10,n)]
            gaps.extend((angles[(i+1)%len(angles)]-angles[i])%(2*math.pi) for i in range(len(angles)))
        cases[name]['max_chord_error_mm'] = max(f['radius'] + (1 if f.get('bulge') else 0) for f in features)*(1-math.cos(max(gaps)/2))
        if rotated:
            origin=transformed((0,0,0)); end=transformed((0,0,1))
            cases[name]['axis']=[end[i]-origin[i] for i in range(3)]
            cases[name]['translation_mm']=origin
    (ROOT/'m2b_manifest.json').write_text(json.dumps({'unit':'mm','cases':cases},ensure_ascii=False,indent=2)+'\n')

if __name__=='__main__': main()
