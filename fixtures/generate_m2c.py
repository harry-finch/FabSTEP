"""Corpus indépendant d'arcs cylindriques ouverts (Python standard)."""
import json
import math
from pathlib import Path

ROOT=Path(__file__).resolve().parent
CENTER=(7.5,51.0)
HEIGHT=18.0


def rotate(p):
    x,y,z=p; a=(1/math.sqrt(14),2/math.sqrt(14),3/math.sqrt(14)); c,s=math.cos(.63),math.sin(.63)
    d=sum(u*v for u,v in zip(a,p)); cross=(a[1]*z-a[2]*y,a[2]*x-a[0]*z,a[0]*y-a[1]*x)
    return tuple(c*v+s*w+(1-c)*d*u+t for v,w,u,t in zip(p,cross,a,(13.,-7.,11.)))


def profile(radius, n, cx=CENTER[0], cy=CENTER[1]):
    # Rectangle [0,30] × [0,60], encoche de coin concave raccordée aux bords.
    top=math.asin((60-cy)/radius); left=-math.acos(-cx/radius)
    arc=[(cx+radius*math.cos(top+(left-top)*i/n),cy+radius*math.sin(top+(left-top)*i/n)) for i in range(n+1)]
    arc[0]=(arc[0][0],60.); arc[-1]=(0.,arc[-1][1])
    # La colonne commune aux deux cellules voisines a 32 segments réguliers.
    return [(0.,0.),(30.,0.)]+[(30.,60*i/32) for i in range(1,33)]+arc


def cross(a,b,c): return (b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0])
def triangulate(points):
    """Oreilles avec points collinéaires retenus sur les frontières communes."""
    ids=list(range(len(points))); out=[]; eps=1e-10
    while len(ids)>3:
        for i,b in enumerate(ids):
            a,c=ids[i-1],ids[(i+1)%len(ids)]
            if cross(points[a],points[b],points[c])<=eps: continue
            if any(cross(points[a],points[b],points[k])>=-eps and cross(points[b],points[c],points[k])>=-eps and cross(points[c],points[a],points[k])>=-eps for k in ids if k not in (a,b,c)): continue
            out.append((a,b,c)); ids.pop(i); break
        else: raise ValueError('Profil non triangulable sans altérer ses frontières')
    out.append(tuple(ids)); return out


class Mesh:
    def __init__(self): self.p=[];self.t=[]
    def ring(self,pts,z):
        ids=[]
        for x,y in pts: ids.append(len(self.p));self.p.append((x,y,z))
        return ids
    def point(self,p): self.p.append(p);return len(self.p)-1
    def tri(self,a,b,c,rev=False): self.t.append((c,b,a) if rev else (a,b,c))
    def wall(self,a,b,rev=False):
        for i in range(len(a)):
            j=(i+1)%len(a);self.tri(a[i],a[j],b[j],rev);self.tri(a[i],b[j],b[i],rev)
    def ringcap(self,outer,inner,rev=False):
        for i in range(len(outer)):
            j=(i+1)%len(outer);self.tri(inner[i],outer[i],outer[j],rev);self.tri(inner[i],outer[j],inner[j],rev)
    def fan(self,ring,center,rev=False):
        c=self.point(center)
        for i in range(len(ring)):self.tri(c,ring[i],ring[(i+1)%len(ring)],rev)
    def write(self,name,rotation):
        points=[rotate(p) for p in self.p] if rotation else self.p
        out=['solid '+name]
        for t in self.t:
            out+=[' facet normal 0 0 0','  outer loop']
            out+=['   vertex '+' '.join(f'{v:.15g}' for v in points[i]) for i in t]
            out+=['  endloop',' endfacet']
        (ROOT/(name+'.stl')).write_text('\n'.join(out+['endsolid '+name])+'\n')


def rectangle(x0,x1,y0,y1,n=16):
    return ([(x1,y0+(y1-y0)*i/n) for i in range(n)]+[(x1-(x1-x0)*i/n,y1) for i in range(n)]+[(x0,y1-(y1-y0)*i/n) for i in range(n)]+[(x0+(x1-x0)*i/n,y0) for i in range(n)])


def build(name,n=36,levels=1,mixed=False,rotation=False,defect=None,radius=18.,hole_only=False):
    mesh=Mesh(); zs=[HEIGHT*i/levels for i in range(levels+1)]
    profiles=[]
    for i,z in enumerate(zs):
        r=radius; cx=CENTER[0]
        if defect=='cone': r+=0.8*i/levels
        if defect=='bulge' and i==levels//2:r+=0.8
        if defect=='mismatch': cx+=0.6*i/levels
        profiles.append(profile(r,n,cx))
    rings=[mesh.ring(pts,z) for pts,z in zip(profiles,zs)]
    for j in range(levels):
        # Toute frontière sauf la jonction droite x=30 (occupée par le reste du bloc).
        for i in range(len(rings[j])):
            k=(i+1)%len(rings[j]);p,q=profiles[j][i],profiles[j][k]
            if p[0]==30. and q[0]==30.:continue
            mesh.tri(rings[j][i],rings[j][k],rings[j+1][k]);mesh.tri(rings[j][i],rings[j+1][k],rings[j+1][i])
    for ids,pts,rev in [(rings[0],profiles[0],True),(rings[-1],profiles[-1],False)]:
        for a,b,c in triangulate(pts):mesh.tri(ids[a],ids[b],ids[c],rev)
    # Deux cellules à droite : cylindre traversant puis bossage ; ou aucune primitive.
    for ymin,ymax,feature in [(0.,30.,'hole' if mixed else None),(30.,60.,'boss' if mixed and not hole_only else None)]:
        box=rectangle(30.,60.,ymin,ymax); cx,cy=45.,(ymin+ymax)/2
        outer=[mesh.ring(box,z) for z in zs]
        for j in range(levels):
            for i,p in enumerate(box):
                k=(i+1)%len(box);q=box[k]
                if (p[0]==q[0]==60.) or (p[1]==q[1] and p[1] in (0.,60.)):
                    mesh.tri(outer[j][i],outer[j][k],outer[j+1][k]);mesh.tri(outer[j][i],outer[j+1][k],outer[j+1][i])
        if feature is None:
            mesh.fan(outer[0],(cx,cy,0.),True);mesh.fan(outer[-1],(cx,cy,HEIGHT))
        else:
            radius_feature=4. if feature=='hole' else 5.
            angles=[math.atan2(y-cy,x-cx) for x,y in box]
            circle=[(cx+radius_feature*math.cos(a),cy+radius_feature*math.sin(a)) for a in angles]
            if feature=='hole':
                inside=[mesh.ring(circle,z) for z in zs];mesh.ringcap(outer[0],inside[0],True);mesh.ringcap(outer[-1],inside[-1]);
                for a,b in zip(inside,inside[1:]):mesh.wall(a,b,True)
            else:
                mesh.fan(outer[0],(cx,cy,0.),True);base=mesh.ring(circle,HEIGHT);top=mesh.ring(circle,HEIGHT+5.)
                mesh.ringcap(outer[-1],base);mesh.wall(base,top);mesh.fan(top,(cx,cy,HEIGHT+5.))
    mesh.write(name,rotation)


def main():
    specs={'m2c_open_arc':dict(), 'm2c_open_arc_subdivided':dict(levels=4), 'm2c_open_arc_rotated':dict(levels=4,rotation=True), 'm2c_open_arc_hole':dict(levels=4,mixed=True,hole_only=True), 'm2c_open_arc_mixed':dict(levels=4,mixed=True), 'm2c_open_arc_mixed_rotated':dict(levels=4,mixed=True,rotation=True), 'm2c_arc_conical':dict(levels=4,defect='cone'), 'm2c_arc_bulge':dict(levels=4,defect='bulge'), 'm2c_arc_mismatch':dict(levels=4,defect='mismatch'), 'm2c_arc_few_segments':dict(n=5), 'm2c_arc_other_angle':dict(n=36,radius=21.)}
    for name,kw in specs.items():build(name,**kw)
    cases={}
    for name,kw in specs.items():
        radius=kw.get('radius',18.); alpha=math.asin(9/radius); beta=-math.acos(-7.5/radius);sweep=alpha-beta
        item={'radius_mm':radius,'arc_sweep_rad':sweep,'height_mm':HEIGHT,'arc_segments':kw.get('n',36),'axial_intervals':kw.get('levels',1),'max_chord_error_mm':radius*(1-math.cos(sweep/(2*kw.get('n',36))))}
        if kw.get('mixed'):
            item['complete_hole_radius_mm']=4.
            if not kw.get('hole_only'):item.update(boss_radius_mm=5.,boss_height_mm=5.)
        if kw.get('rotation'): item.update(rotated=True,translation_mm=rotate((0,0,0)),axis=[rotate((0,0,1))[i]-rotate((0,0,0))[i] for i in range(3)])
        if kw.get('defect'):item['negative']=kw['defect']
        if kw.get('n')==5:item['negative']='six points : échantillonnage insuffisant ; tester avec budget 1 mm'
        if not kw.get('defect'):
            # Aire du rectangle retranchée de la région circulaire limitée par les côtés x=0 et y=60.
            arc=profile(radius,kw.get('n',36)); area=abs(sum(arc[i][0]*arc[(i+1)%len(arc)][1]-arc[(i+1)%len(arc)][0]*arc[i][1] for i in range(len(arc)))/2)
            polygon_arc_area=0.5*radius**2*(sweep-kw.get('n',36)*math.sin(sweep/kw.get('n',36)))
            item['nominal_volume_mm3']=(area+1800-polygon_arc_area)*HEIGHT + ((math.pi*((0 if kw.get('hole_only') else 25*5)-16*HEIGHT)) if kw.get('mixed') else 0)
            item['nominal_faces']=7+(1 if kw.get('mixed') else 0)+(2 if kw.get('mixed') and not kw.get('hole_only') else 0)
            item['nominal_cylindrical_faces']=1+(1 if kw.get('mixed') else 0)+(1 if kw.get('mixed') and not kw.get('hole_only') else 0)
            item['note_budget']='Un budget inférieur à max_chord_error_mm doit refuser la reconstruction analytique.'
        cases[name]=item
    (ROOT/'m2c_manifest.json').write_text(json.dumps({'unit':'mm','cases':cases},ensure_ascii=False,indent=2)+'\n')

if __name__=='__main__':main()
