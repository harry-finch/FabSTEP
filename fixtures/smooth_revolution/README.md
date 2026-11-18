# Révolutions cubiques à deux disques

Le générateur [`generate-smooth-revolution-fixtures.py`](../../scripts/generate-smooth-revolution-fixtures.py) construit les STL à partir d’un méridien de Bézier en millimètres. Les quatre points de contrôle ont pour coordonnées axiales `0`, `H/3`, `2H/3` et `H`. Leurs coordonnées radiales définissent le rayon sur un tour complet. Deux éventails de triangles ferment les anneaux d’extrémité. La paroi échantillonne des niveaux axiaux et des azimuts communs, reliés par des diagonales alternées.

| Cas | Paramètres de référence |
| --- | --- |
| `bulge` | Profil bombé, rayons de contrôle `[5, 9, 9, 5]`, hauteur 20 mm |
| `neck` | Profil étranglé, rayons `[8, 3, 3, 8]`, hauteur 20 mm |
| `inflection` | Point d’inflexion, rayons `[4, 9, 3, 8]`, hauteur 24 mm |
| `rotated` | `inflection` tourné sur un axe oblique et translaté, export STL ASCII pour préserver la planéité numérique des disques |
| `scaled` | `bulge` agrandi de 50 % |
| `mirrored` | `inflection` réfléchi axialement et translaté |
| `retessellated` | `neck` à 40 divisions axiales et 80 azimuts avec un déphasage angulaire, contre 32 et 64 pour les autres cas valides |

`missing_cap` omet le disque supérieur. `folded` déplace un sommet médian au-delà de la colonne suivante, à rayon et hauteur constants. La carte angulaire se replie alors que les sommets suivent toujours le méridien cubique. `incomplete_level` déplace un sommet médian en hauteur sur le même méridien et laisse un anneau incomplet. `sparse` réduit la paroi à trois divisions axiales. `quartic` ajoute `64 t² (1−t)²` au rayon de `bulge`, avec les mêmes extrémités. `axial_tip` ferme le corps par une pointe existante au lieu du disque inférieur, avec un profil cubique de rayons `[0, 4, 2, 8]`.

Le générateur relit les STL, contrôle l’orientation des arêtes, la connexité des faces, les liens de sommets et la caractéristique d’Euler. Le manifeste associe les triangles source à leurs trois régions et encode les sondes intérieures et extérieures. Le volume de référence provient d’une intégration de Simpson sur les sections circulaires. Les bornes analytiques évaluent les extrémités et les points stationnaires de chaque projection axiale et radiale. Les contrôles de topologie portent sur la structure combinatoire, y compris dans le cas replié.

Régénération depuis la racine avec `python3 scripts/generate-smooth-revolution-fixtures.py`.
