# Méridiennes cubiques sur un maillage irrégulier

Ces STL proviennent de deux solides dessinés et maillés dans FreeCAD. Le premier tourne un profil de 20 mm de haut dont les rayons de Bézier sont (5, 9, 9, 5) mm. Le second utilise (4, 9, 3, 8) mm sur 24 mm, puis une rotation et une translation. Chaque paroi est fermée par deux disques réels. Les triangles ne sont pas tous délimités par des anneaux source complets.

La reconstruction commence par le contrôle historique des anneaux complets. En cas de maillage irrégulier, elle exige des sommets hors anneaux séparés d’au moins un huitième de hauteur, des témoins sur toute la hauteur et tout autour de l’axe. Au moins onze mesures distinctes servent au choix et au contrôle du profil. Tous les sommets latéraux doivent ensuite respecter le rayon du profil. Chaque triangle de la paroi doit se projeter dans le bon sens sur le tour complet en angle et hauteur.

Sur un triangle du maillage, la projection conserve la hauteur et l’angle du point. Le contrôle borne l’écart radial par la somme des résidus aux sommets, de la flèche angulaire et de l’erreur d’interpolation scalaire du rayon selon la hauteur. Les cellules trop larges sont évaluées sur quatre sous-triangles du même triangle plan, jusqu’à trois subdivisions. Le solide, le volume, les disques et la méridienne sont contrôlés après réimport du STEP.

| Fichier | Triangles | Attendu avec 0,05 mm |
| --- | ---: | --- |
| `bulged-coarse.stl` | 4 428 | Deux plans et une révolution |
| `bulged-fine.stl` | 3 760 | Deux plans et une révolution |
| `inflected-coarse.stl` | 3 584 | Deux plans et une révolution |
| `inflected-fine.stl` | 4 960 | Reconnaissance refusée. La projection radiale s’écarte localement du profil de plus de 0,086 mm. |

Les suffixes identifient les deux réglages angulaires du mailleur FreeCAD. Pour régénérer les STL, lancer `scripts/qualify-irregular-meridians-freecad.py DOSSIER --mesh-dir SORTIE_STL` avec le Python de FreeCAD. Après export FabSTEP dans `DOSSIER`, ajouter `--check` pour contrôler le volume, le solide et 165 positions par résultat positif.

## Deux travées tangentes sans anneau au raccord

`two_span-{coarse,fine}.stl` et `two_span_turned-{coarse,fine}.stl` proviennent de deux B-splines de degré trois à un nœud double. Le raccord est situé à 9 mm sur 24 pour le premier dessin, à 18 mm sur 30 pour le second, retourné et translaté. Les quatre maillages doivent produire un solide à deux disques et à une seule paroi, dont la méridienne conserve le nœud intérieur. `kinked-coarse.stl` remplace la jonction tangente du premier dessin par une cassure de pente. Son ajustement lisse doit échouer.

Après le refus de la mono-travée, le moteur examine des couples de hauteurs témoins consécutives. Chaque côté doit apporter au moins sept hauteurs échantillonnées, dont quatre définissent un polynôme et les autres le contrôlent. La possibilité de faire varier chaque rayon de la tolérance d’ajustement fournit une enveloppe de Lagrange sur la valeur et les deux premières dérivées. Un intervalle peut contenir le nœud si la différence des dérivées y change de signe de manière résolue et si la différence des courbures garde le même signe sur tout l’intervalle. Plusieurs intervalles possibles, une dérivée indéterminée, une courbure non résolue ou une discontinuité de rayon bloquent cet ajustement. Le moteur impose alors la tangente commune et contrôle de nouveau l’ensemble des sommets et des triangles, y compris ceux qui franchissent le nœud. Le raccord doit se situer entre un huitième et sept huitièmes de la hauteur.
