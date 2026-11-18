# Poche borgne à un congé de fond arrêté

Ce corpus public décrit un bloc rectangulaire usiné par le dessus. Un quart de cylindre concave relie le fond plat de la poche à une seule paroi latérale. Ses deux extrémités forment des arcs sur les parois de butée. Les trois autres arêtes du fond sont vives. Le solide de référence comporte onze faces planes et une face cylindrique.

Le bloc nominal mesure 30 × 20 × 12 mm. La poche mesure 16 × 10 mm à l’ouverture et descend jusqu’à 3 mm au-dessus du dessous. Le congé a un rayon de 2 mm sur une longueur de 16 mm. Son volume de matière supplémentaire par rapport à une poche à angles vifs vaut `longueur × rayon² × (1 − π/4)`. Le solide idéal a donc un volume de 5 773,734517543 mm³.

| Fichier | Géométrie et maillage |
| --- | --- |
| `nominal.stl` | Seize intervalles angulaires et deux bandes axiales. |
| `rotated.stl` | Même pièce après deux rotations et une translation, en STL ASCII à 17 chiffres significatifs. |
| `rotated_f32.stl` | Même pièce, même placement et mêmes triangles, encodés en STL binaire à coordonnées f32. Ce témoin mesure l’effet de la précision aux jonctions des parois. |
| `retessellated.stl` | Vingt-trois intervalles angulaires, trois bandes axiales et diagonales alternées. |
| `opposite.stl` | Bloc de 38 × 24 × 14 mm et poche de 20 × 14 mm. Le congé de rayon 3 mm occupe le côté opposé. |
| `staggered.stl` | Seize intervalles à une extrémité et vingt-trois à l’autre. Trente-neuf triangles relient les sommets en ordre angulaire. |
| `missing_floor.stl` | Plancher de la poche retiré. Le maillage possède six arêtes ouvertes. |
| `warped.stl` | Sommet intermédiaire du cylindre décalé de 0,16 mm dans la direction radiale. Les deux arcs d’arrêt sont intacts. |
| `warped_stop.stl` | Même discrétisation que `staggered`, avec un sommet décalé de 0,16 mm sur le second arc et sa paroi de butée. Le maillage reste fermé. |

Le générateur Python utilise uniquement la bibliothèque standard. Il vérifie les incidences orientées des arêtes après encodage et compare le volume du maillage fermé de chaque référence positive à la formule géométrique, en comptant l’effet des cordes. `manifest.json` fournit pour chaque surface l’intervalle d’indices de ses triangles dans le fichier (début inclus, fin exclue). Pour les témoins de la pièce idéale, il indique aussi les bornes et centres des extrémités en coordonnées monde, l’aire du congé et deux points de contrôle dans le vide de la poche et dans la matière du raccord.

Depuis la racine du dépôt, `python3 scripts/generate-pocket-fillet-fixtures.py` régénère les STL et le manifeste. Un chemin en argument permet de choisir un autre répertoire.
