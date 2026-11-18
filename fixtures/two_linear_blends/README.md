# Deux congés de rayon variable

Deux congés extérieurs occupent des arêtes parallèles diagonalement opposées d’un bloc de 20 × 16 × 10 mm. Les deux rayons aux extrémités de chaque arête fixent une loi affine. Leurs enveloppes coniques se terminent sur les deux mêmes plans de coupe et sur quatre plans latéraux distincts. Les retraits maximaux des deux arêtes doivent, ensemble, être inférieurs à la largeur et à la profondeur du bloc avec une marge résolue.

| Dessin | Rayons au bas et au haut de l’arête, mm | Placement |
| --- | --- | --- |
| `opposed` | 3 → 5 et 4 → 2,5 | Origine du bloc |
| `offset` | 2 → 4,4 et 2,5 → 3,7 | Permutation des axes et translation |

`generate-two-linear-blends.py` écrit les STL du premier corpus et leur manifeste. Celui-ci associe chaque triangle à son plan ou à sa paroi conique. `nonlinear_second.stl` déforme le second congé de 0,2 mm à mi-hauteur et conserve ses deux limites terminales. Une tentative de reconnaissance des deux congés échoue sur ce témoin.

Le corpus `freecad/` provient de deux dessins indépendants par découpes booléennes. Chaque dessin est maillé deux fois avec FreeCAD à une déflexion linéaire de 0,02 mm et à des déflexions angulaires de 0,18 et 0,07 rad. Pour reproduire le contrôle, exporter les deux STL nominaux avec FabSTEP vers `opposed.step` et `offset.step` dans un dossier, puis exécuter `scripts/qualify-two-linear-blends-freecad.py DOSSIER_DES_STEP --mesh-dir DOSSIER_DES_STL` avec le Python de FreeCAD. Le script y écrit les références, contrôle la topologie, le volume et la matière, puis produit les maillages si le dossier STL est indiqué.

La preuve de paroi interpole pour chaque triangle la hauteur et la quantité `rayon × angle`. Le rayon est positif et affine en hauteur. Les coordonnées de la paroi couvrent un trapèze borné par les deux ellipses et les deux génératrices. Le contrôle de l’orientation et de l’aire dans ce trapèze assure sa couverture entière. L’écart continu s’obtient par la borne de corde de la section elliptique, multipliée par le rayon maximal du triangle. La somme tient également compte des projections, des plans et du soudage. Les raccords entre congés, les rayons non linéaires et les contacts singuliers demandent d’autres preuves.
