# Poche à deux congés de fond opposés

Le fond de la poche est relié à ses deux parois longitudinales par deux quarts de cylindre concaves. Les deux axes sont parallèles. Un fond plan sépare les congés, dont les quatre arcs de cercle se terminent sur les deux butées droites. Le solide idéal possède onze faces planes et deux faces cylindriques.

| STL | Cas mesuré |
| --- | --- |
| `nominal.stl` | Bloc de 30 × 20 × 12 mm. La poche de 16 × 10 mm descend jusqu’à 3 mm du dessous. Les rayons de 2 et 3 mm laissent 5 mm de fond plat. Deux bandes axiales, seize et vingt intervalles angulaires. |
| `staggered.stl` | Même géométrie. Deux bandes relient trois rangées échantillonnées indépendamment. Elles comptent 16, 23 et 30 intervalles à gauche, 20, 27 et 34 à droite. |
| `rotated.stl` | Poche de 18 × 12 mm, rayons de 1,5 et 3,5 mm. Trois bandes à diagonales alternées. La pièce est tournée puis translatée et l’axe des congés est parallèle à Y. |
| `warped_stop.stl` | Sur une seule bande à subdivisions décalées, un sommet de l’arc d’arrêt du congé droit est écarté radialement de 0,16 mm. Le maillage reste fermé et le congé gauche conserve sa géométrie. |

Pour un congé de rayon `r` et de longueur `L`, l’arrondi conserve `L × r² × (1 − π/4)` de matière. Les deux contributions s’ajoutent au volume du bloc percé d’une poche à angles vifs. Le volume solide attendu vaut 5 804,637182013 mm³ sur le témoin nominal et 5 312,011079353 mm³ sur la pièce tournée.

Depuis la racine, `python3 scripts/generate-opposed-pocket-fixtures.py` régénère les quatre STL et le manifeste. Un répertoire de sortie facultatif permet de les reproduire ailleurs. Le générateur relit les coordonnées binaires, vérifie que chaque arête orientée possède un seul voisin de sens inverse, et borne l’écart entre volume maillé et volume analytique des témoins positifs.

Les intervalles de triangles sont donnés dans le manifeste, début inclus et fin exclue. Pour retrouver chaque cylindre, utiliser `groups.left_fillet` ou `groups.right_fillet`. Le tableau `fillets` suit le même ordre et donne les rayons, les centres des arcs, les aires et les sondes de matière dans le repère de la pièce placée.
