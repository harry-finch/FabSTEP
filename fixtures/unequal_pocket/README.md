# Poche à deux congés adjacents de rayons différents

Le maillage décrit une poche borgne rectangulaire dont deux arêtes du fond portent des congés cylindriques concaves. Le rayon de chaque congé est constant. Leur branche commune commence sur le fond et finit au niveau de tangence du petit cylindre. Au-delà, le grand cylindre partage un arc de cercle avec la paroi verticale voisine. Chaque cylindre se termine sur un quart de cercle à l’autre bout de la poche. La pièce analytique comporte onze plans et deux cylindres.

| Fichier | Géométrie | Référence |
| --- | --- | --- |
| `nominal.stl` | Poche de 16 × 10 mm, rayons de 2 mm autour de X et 3 mm autour de Y. 416 triangles. | Volume de 5 791,673203086 mm³. La branche commune comporte 18 segments dans le STL, le prolongement circulaire en compte 4. |
| `reflected.stl` | Poche de 18 × 12 mm, rayons locaux de 3,5 et 1,5 mm. Réflexion, permutation des axes, translation et orientation des triangles corrigée. 912 triangles. | Volume de 5 308,068388490 mm³, 24 segments sur la branche, 10 sur le prolongement du grand cylindre. |
| `warped_seam.stl` | Un sommet intérieur de la branche nominale est déplacé de 0,14 mm selon X et Y, y compris dans toutes ses facettes incidentes. | La coque reste fermée, mais le sommet quitte les deux cylindres théoriques. |

Pour une hauteur `h` au-dessus du fond, le recul sur une paroi de rayon `r` vaut `q(r,h) = r − √(h(2r−h))` jusqu’à `h = r`, puis zéro. La section horizontale du vide est un rectangle de dimensions `L − q(ry,h)` par `W − q(rx,h)`. Son sommet commun décrit la jonction des deux cylindres jusqu’à la hauteur du petit rayon. Dans le témoin nominal, le quatrième point de contrôle est à 0,013124 mm du plan défini par les premier, troisième et cinquième points. Le manifeste fournit ces points dans le repère final, les centres des cercles d’arrêt, les extrémités de l’arc de prolongement et les intervalles de triangles de chaque face.

Le générateur calcule le volume théorique en intégrant la section horizontale par primitives et vérifie le terme de recouvrement par quadrature indépendante. Il relit les coordonnées binaires f32 pour contrôler la fermeture orientée, la longueur des chaînes d’arêtes partagées et l’écart de volume dû aux cordes. Sur les deux témoins positifs, les volumes obtenus dans FreeCAD par intersection de deux outils à congé unique diffèrent des références intégrées de moins de 0,0002 mm³.

Depuis la racine du projet, lancer `python3 scripts/generate-unequal-pocket-fixtures.py`. Un répertoire de sortie facultatif permet de reproduire les trois STL et le manifeste ailleurs.
