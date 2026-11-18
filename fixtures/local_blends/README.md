# Corpus source indépendant D et G

`python3 scripts/generate-local-blend-fixtures.py` génère les STL et `manifest.json` avec la bibliothèque standard Python. Le script relit les STL écrits et vérifie les incidences orientées des arêtes, les liens des sommets, la caractéristique d’Euler, les sondes matière/vide et la correspondance des bornes des maillages avec leurs références. Les empreintes SHA-256 et les intervalles d’attribution des triangles figurent dans le manifeste. Les rotations obliques utilisent le STL ASCII en double précision. Les autres variantes utilisent le STL binaire.

## D — Coin triédrique

Bloc 20 × 16 × 10 mm, rayon 2 mm. Une grille barycentrique normalisée donne l’octant sphérique. Ses trois bords sont réemployés à l’identique au départ des cylindres. Les arrêts dans les trois plans distants contiennent les mêmes sommets d’arcs que les cylindres. Les bandes et les plans de tangence partagent aussi les sommets de leurs génératrices. Le modèle nominal comprend six plans, trois cylindres partiels et un octant, avec un volume analytique de **3 161,852496348378 mm³**.

La perte de volume se décompose en trois bandes de section `r²(1−π/4)` sur une longueur totale `L+W+H−3r`, puis un cube de côté `r` auquel on conserve un octant sphérique. Le corpus contient le nominal, son miroir, une permutation d’axes, une rotation oblique avec une autre tessellation, un sommet intérieur déplacé et un plan de tangence absent.

## G — Congé à rayon linéaire

Dans les repères locaux, les centres des sphères suivent `C(t)=(a+bt,a+bt,t)`. Pour un point de l’enveloppe à hauteur `z`, la condition de stationnarité donne `t−z=b(x+y−a−bz)/(1+b²)`. La substitution dans l’équation de sphère donne `(x+y−a−bz)²/(1+b²)=2xy`. Sur la branche physique, la section horizontale vérifie `x+y+√[2(1+b²)xy]=a+bz` entre les deux plans tangents.

Le paramètre `s` parcourt `[0,1]`. Avec `k=√[2(1+b²)]` et `D=(1−s)²+s²+ks(1−s)`, les rapports `x/(a+bz)` et `y/(a+bz)` valent respectivement `(1−s)²/D` et `s²/D`. Le générateur vérifie chaque point par la sphère stationnaire et le cône circulaire incliné. Les sommets de coupe et de tangence sont réemployés directement sur les plans voisins.

Pour les lois `3+0,2z` et `5−0,2z`, le volume analytique est **3 165,401595323511 mm³**. L’aire retirée par section est proportionnelle à `(a+bz)²`. Le coefficient analytique est vérifié numériquement par l’intégrale de Green de la paramétrisation rationnelle. Les variantes comprennent les deux signes de pente, un miroir, une permutation, une rotation oblique avec triangulation décalée, un empilement de quarts de cercle, une loi non linéaire, une coupe limite parabolique et un arrêt absent.

Les trois sondes discriminantes de l’empilement circulaire sont dans la matière de l’enveloppe linéaire et dans le vide de l’empilement. Ces contrôles portent sur les STL du corpus. Les sources G servent à la qualification future de R-082. Les tests d’intégration D2 se trouvent dans `crates/fabstep-app/tests/local_trihedral.rs`.

Les témoins `g_ruled_strip.stl` et `g_ruled_decreasing.stl` portent uniquement des génératrices reliant les deux coupes. `g_ruled_circle.stl` partage cette triangulation avec des sections circulaires. Reconstruction et contrôles dans V-1222.
