"""Génère un 3MF indépendant via FreeCAD Mesh, avec deux objets placés.

Exécuter avec le Python de FreeCAD ; aucune dépendance à FabSTEP.
Les deux cubes mesurent 1 mm ; le second est translaté de (3, 2, 0) mm.
"""
from pathlib import Path
import FreeCAD
import Mesh

root = Path(__file__).resolve().parent
doc = FreeCAD.newDocument("FabSTEPM3Reference")
first = doc.addObject("Mesh::Feature", "First")
first.Mesh = Mesh.Mesh(str(root / "cube.stl"))
second = doc.addObject("Mesh::Feature", "Second")
second.Mesh = Mesh.Mesh(str(root / "cube.stl"))
second.Placement.Base = FreeCAD.Vector(3, 2, 0)
Mesh.export([first, second], str(root / "m3_freecad_instances.3mf"))
FreeCAD.closeDocument(doc.Name)
