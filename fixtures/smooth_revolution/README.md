# Révolutions cubiques

Le générateur [`generate-smooth-revolution-fixtures.py`](../../scripts/generate-smooth-revolution-fixtures.py) construit les STL à partir d’un méridien de Bézier en millimètres. Les quatre points de contrôle ont pour coordonnées axiales `0`, `H/3`, `2H/3` et `H`. Leurs coordonnées radiales définissent le rayon sur un tour complet. Les extrémités sont deux disques, ou un disque et un pôle axial. La paroi échantillonne des niveaux axiaux et des azimuts communs, reliés par des diagonales alternées.

| Cas | Paramètres de référence |
| --- | --- |
| `bulge` | Profil bombé, rayons de contrôle `[5, 9, 9, 5]`, hauteur 20 mm |
| `neck` | Profil étranglé, rayons `[8, 3, 3, 8]`, hauteur 20 mm |
| `inflection` | Point d’inflexion, rayons `[4, 9, 3, 8]`, hauteur 24 mm |
| `rotated` | `inflection` tourné sur un axe oblique et translaté, export STL ASCII pour préserver la planéité numérique des disques |
| `scaled` | `bulge` agrandi de 50 % |
| `mirrored` | `inflection` réfléchi axialement et translaté |
| `axial_tip` | Pointe sur l’axe, rayons de contrôle `[0, 4, 2, 8]`, hauteur 20 mm, 64 triangles incidents à la pointe |
| `mirrored_tip` | Autre pointe, réfléchie et translatée, rayons de contrôle `[0, 2, 5, 4.5]`, hauteur 16 mm |
| `retessellated` | `neck` à 40 divisions axiales et 80 azimuts avec un déphasage angulaire, contre 32 et 64 pour les autres cas valides |

`missing_cap` omet le disque supérieur. `folded` déplace un sommet médian au-delà de la colonne suivante, à rayon et hauteur constants. La carte angulaire se replie alors que les sommets suivent toujours le méridien cubique. `incomplete_level` déplace un sommet médian en hauteur sur le même méridien et laisse un anneau incomplet. `sparse` réduit la paroi à trois divisions axiales. `quartic` ajoute `64 t² (1−t)²` au rayon de `bulge`, avec les mêmes extrémités. `off_axis_pole` déplace la pointe réelle de `axial_tip` de 0,02 mm hors de l’axe. Cette pièce reste fermée, mais sa pointe interdit la reconstruction cubique.

Le générateur relit les STL, contrôle l’orientation des arêtes, la connexité des faces, les liens de sommets et la caractéristique d’Euler. Le manifeste associe les triangles source à leurs régions et encode les sondes intérieures et extérieures. Le volume de référence provient d’une intégration de Simpson sur les sections circulaires. Les bornes analytiques évaluent les extrémités et les points stationnaires de chaque projection axiale et radiale. Les contrôles de topologie portent sur la structure combinatoire, y compris dans le cas replié.

Régénération depuis la racine avec `python3 scripts/generate-smooth-revolution-fixtures.py`.


Les fichiers `two_span.stl` et `two_span_mirror.stl` décrivent une pièce de 24 mm de haut. Deux cubiques positives se rejoignent tangentiellement à 9 mm. Leurs rayons de contrôle valent respectivement (6, 4,5, 7,5, 8) mm et (8, 8 + 5/6, 4, 6) mm. Le deuxième STL utilise un placement réfléchi et un autre nombre de triangles. Dans `kinked_span.stl`, le deuxième contrôle de la seconde cubique est ramené à 7 mm. La cassure de tangente interdit le modèle lisse. La relecture indépendante des exports `two_span.step` et `two_span_mirror.step` se lance avec le Python de FreeCAD et `scripts/validate-cubic-spans-freecad.py DOSSIER_DES_STEP`.

Pour les deux pointes, la paroi inclut tous les triangles de l’éventail source. Après réimport du STEP, la limite paramétrique de la paroi se réduit à une arête dégénérée à un seul sommet, dont la position est comparée à celle de la pointe source. La relecture indépendante de `axial_tip.step` et `mirrored_tip.step` utilise `scripts/validate-cubic-spans-freecad.py DOSSIER_DES_STEP --poles`.
