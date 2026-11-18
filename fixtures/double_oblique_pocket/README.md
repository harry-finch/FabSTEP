# Congé de poche entre deux butées obliques

Ces trois STL binaires sont issus d’un dessin indépendant dans FreeCAD. Une poche borgne présente un congé de rayon 2,5 mm, partagé avec deux faces planes tangentes. Les deux butées coupent le cylindre en arcs elliptiques.

| Fichier | Géométrie | Triangles |
|---|---|---:|
| `opposed.stl` | Deux butées de pentes opposées | 108 |
| `parallel.stl` | Deux butées parallèles inclinées suivant deux directions, pièce en miroir | 132 |
| `compound.stl` | Deux plans de normales différentes, pièce tournée de 17° | 142 |

Le [générateur et lecteur indépendant](../../scripts/qualify-double-oblique-pocket-freecad.py) crée les dessins et les maillages et peut aussi contrôler les STEP exportés par FabSTEP. Un dessin donne onze plans et un cylindre concave. Les deux arcs elliptiques correspondent chacun à un bord partagé par la paroi et une butée.
