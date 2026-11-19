# Deux congés à rayon variable sur des arêtes gauches

Le bloc mesure 20 × 16 × 10 mm. Une première arête est parallèle à Z, à X = Y = 0. La seconde est parallèle à Y, à X = 20 mm et Z = 0. Chacune porte sa propre loi affine de rayon. Les deux supports opposés en X séparent complètement les deux zones de matière retirées.

| Modèle | Rayon sur Z (mm) | Rayon sur Y (mm) | Volume du dessin (mm³) |
| --- | --- | --- | --- |
| vertical | 2 + 0,15 z | 4 − 0,125 y | 3 151,607063 |
| swapped | 3,5 − 0,2 z | 1,5 + 0,1875 y | 3 152,958225 |

Les fichiers coarse et fine sont deux tessellations du STEP de référence, avec des déflexions angulaires de 0,18 et 0,07 rad. Dans le témoin notched, une encoche coupe une des parois coniques. Les tests vérifient aussi un déplacement ponctuel de 0,08 mm et une réflexion rigide avec permutation des triangles.

Le script construit les solides avec FreeCAD, contrôle leurs sections et leurs volumes, puis compare les exports FabSTEP au dessin indépendant. Les quatre arcs elliptiques sont mesurés sur les deux cônes après réimport.

```sh
cargo build --offline -p fabstep-cli
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/qualify-skew-linear-blends-freecad.py /tmp/skew-blends \
  --cli target/debug/fabstep --require-analytic
```
