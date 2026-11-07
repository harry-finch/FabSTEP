"""Corpus 3MF Core/OPC indépendant, généré par Python standard."""
from pathlib import Path
from xml.sax.saxutils import escape
import json
import zipfile

ROOT=Path(__file__).resolve().parent
CORE='http://schemas.microsoft.com/3dmanufacturing/core/2015/02'
REL='http://schemas.openxmlformats.org/package/2006/relationships'
TYPES='http://schemas.openxmlformats.org/package/2006/content-types'
MODEL_TYPE='application/vnd.ms-package.3dmanufacturing-3dmodel+xml'
MODEL_REL='http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel'
POINTS=[(0,0,0),(10,0,0),(10,10,0),(0,10,0),(0,0,10),(10,0,10),(10,10,10),(0,10,10)]
TRIS=[(0,2,1),(0,3,2),(4,5,6),(4,6,7),(0,1,5),(0,5,4),(1,2,6),(1,6,5),(2,3,7),(2,7,6),(3,0,4),(3,4,7)]

def mesh_object(id, points=POINTS, triangles=TRIS, name='Cube'):
    vertices=''.join(f'<vertex x="{x}" y="{y}" z="{z}"/>' for x,y,z in points)
    tris=''.join(f'<triangle v1="{a}" v2="{b}" v3="{c}"/>' for a,b,c in triangles)
    return f'<object id="{id}" type="model" name="{escape(name)}"><mesh><vertices>{vertices}</vertices><triangles>{tris}</triangles></mesh></object>'

def components(id, components):
    entries=''.join(f'<component objectid="{obj}"'+(f' transform="{tr}"' if tr else '')+'/>' for obj,tr in components)
    return f'<object id="{id}" type="model"><components>{entries}</components></object>'

def model(objects, items, unit='millimeter', required=False):
    attrs=f' xmlns="{CORE}" xml:lang="fr-FR"'
    if unit is not None: attrs+=f' unit="{unit}"'
    if required: attrs+=' xmlns:x="https://example.invalid/unsupported/2026" requiredextensions="x"'
    build=''.join(f'<item objectid="{obj}"'+(f' transform="{tr}"' if tr else '')+'/>' for obj,tr in items)
    return f'<?xml version="1.0" encoding="UTF-8"?><model{attrs}><resources>{"".join(objects)}</resources><build>{build}</build></model>'

def write(name, xml, extra=None):
    types=f'<?xml version="1.0" encoding="UTF-8"?><Types xmlns="{TYPES}"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="model" ContentType="{MODEL_TYPE}"/></Types>'
    rel=f'<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="{REL}"><Relationship Id="rel-1" Type="{MODEL_REL}" Target="/3D/3dmodel.model"/></Relationships>'
    with zipfile.ZipFile(ROOT/(name+'.3mf'),'w',compression=zipfile.ZIP_DEFLATED,compresslevel=9) as z:
        for path,content in [('[Content_Types].xml',types),('_rels/.rels',rel)]:
            info=zipfile.ZipInfo(path,(2020,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,content.encode())
        info=zipfile.ZipInfo('3D/3dmodel.model',(2020,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED
        if extra:
            # Modèle XML valide : 70 Mio d'espaces compressibles, à traiter comme entrée non fiable.
            prefix, suffix=xml.split('<resources>',1)
            with z.open(info,'w') as dst:
                dst.write(prefix.encode())
                for _ in range(70):dst.write(b' '*(1024*1024))
                dst.write(('<resources>'+suffix).encode())
        else:
            z.writestr(info,xml.encode())

I='1 0 0 0 1 0 0 0 1 0 0 0'
def translate(x,y,z):return f'1 0 0 0 1 0 0 0 1 {x} {y} {z}'

def main():
    cases={}
    def add(name,xml,expected,extra=None):write(name,xml,extra);cases[name]=expected
    add('m3_single_cube',model([mesh_object(1)],[(1,None)]),dict(valid=True,unit='millimeter',solids=1,body_bounds_mm=[[[0,0,0],[10,10,10]]],volume_each_mm3=[1000]))
    add('m3_default_unit',model([mesh_object(1)],[(1,None)],unit=None),dict(valid=True,unit='millimeter',unit_default=True,solids=1,body_bounds_mm=[[[0,0,0],[10,10,10]]],volume_each_mm3=[1000]))
    add('m3_instances',model([mesh_object(1)],[(1,None),(1,translate(30,5,0))]),dict(valid=True,unit='millimeter',solids=2,body_bounds_mm=[[[0,0,0],[10,10,10]],[[30,5,0],[40,15,10]]],volume_each_mm3=[1000,1000]))
    # Matrices 3MF ligne-vecteur : translation locale (2,0), rotation 90°, translation globale (20,10).
    rotation='0 1 0 -1 0 0 0 0 1 0 0 0'
    nested=[mesh_object(1),components(2,[(1,translate(2,0,0))]),components(3,[(2,rotation)])]
    add('m3_nested_components',model(nested,[(3,translate(20,10,0))]),dict(valid=True,unit='millimeter',solids=1,body_bounds_mm=[[[10,12,0],[20,22,10]]],volume_each_mm3=[1000]))
    inch_pts=[tuple(v/10 for v in p) for p in POINTS]
    add('m3_inches',model([mesh_object(1,inch_pts)],[(1,translate(2,0,0))],unit='inch'),dict(valid=True,unit='inch',solids=1,body_bounds_mm=[[[50.8,0,0],[76.2,25.4,25.4]]],volume_each_mm3=[25.4**3]))
    affine='2 0 0 0.25 3 0 0 0 0.5 5 -2 7'
    add('m3_affine',model([mesh_object(1)],[(1,affine)]),dict(valid=True,unit='millimeter',solids=1,body_bounds_mm=[[[5,-2,7],[27.5,28,12]]],volume_each_mm3=[3000],note='échelle non uniforme et cisaillement'))
    mirror='-1 0 0 0 1 0 0 0 1 20 0 0'
    add('m3_mirror',model([mesh_object(1)],[(1,mirror)]),dict(valid=True,unit='millimeter',solids=1,negative_determinant=True,body_bounds_mm=[[[10,0,0],[20,10,10]]],volume_each_mm3=[1000]))
    pts=POINTS+[(x+30,y,z) for x,y,z in POINTS];tris=TRIS+[tuple(i+8 for i in t) for t in TRIS]
    add('m3_disconnected_mesh',model([mesh_object(1,pts,tris)],[(1,None)]),dict(valid=True,unit='millimeter',solids=2,body_bounds_mm=[[[0,0,0],[10,10,10]],[[30,0,0],[40,10,10]]],volume_each_mm3=[1000,1000],note='un mesh, deux composantes fermées distinctes'))
    add('m3_overlap',model([mesh_object(1)],[(1,None),(1,translate(5,0,0))]),dict(valid=False,reason='deux solides se chevauchent ; pas de fusion silencieuse',expected_error='cad.bodies_not_separated'))
    add('m3_touching',model([mesh_object(1)],[(1,None),(1,translate(10,0,0))]),dict(valid=False,reason='deux solides partagent une face',expected_error='cad.bodies_not_separated'))
    nested_cube=[tuple(v/5 for v in p) for p in POINTS]
    add('m3_nested_solids',model([mesh_object(1),mesh_object(2,nested_cube,name='Petit cube')],[(1,None),(2,translate(4,4,4))]),dict(valid=False,reason='un solide est inclus dans un autre',expected_error='cad.bodies_not_separated'))
    add('m3_cycle',model([components(1,[(2,None)]),components(2,[(1,None)])],[(1,None)]),dict(valid=False,reason='cycle de composants et référence avant définition'))
    add('m3_bad_indices',model([mesh_object(1,triangles=TRIS[:-1]+[(0,1,999)])],[(1,None)]),dict(valid=False,reason='indice de sommet hors plage'))
    add('m3_open_mesh',model([mesh_object(1,triangles=TRIS[:-1])],[(1,None)]),dict(valid=False,reason='maillage ouvert'))
    add('m3_required_extension',model([mesh_object(1)],[(1,None)],required=True),dict(valid=False,reason='extension requise non prise en charge'))
    add('m3_zip_budget',model([mesh_object(1)],[(1,None)]),dict(valid=False,reason='partie modèle XML avec 70 Mio décompressés ; plafond 64 Mio par partie',uncompressed_padding_bytes=70*1024*1024),extra=True)
    cases['m3_freecad_instances']=dict(valid=True,unit='millimeter',solids=2,body_bounds_mm=[[[0,0,0],[1,1,1]],[[3,2,0],[4,3,1]]],volume_each_mm3=[1,1],producer='FreeCAD 1.1.1 — generate_freecad_m3.py')
    (ROOT/'m3_manifest.json').write_text(json.dumps({'units':'mm sauf mention','cases':cases},ensure_ascii=False,indent=2)+'\n')

if __name__=='__main__':main()
