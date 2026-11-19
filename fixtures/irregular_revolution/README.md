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
