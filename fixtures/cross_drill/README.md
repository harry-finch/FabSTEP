# Corpus de perçages cylindriques orthogonaux

`python3 scripts/generate-cross-drill-fixtures.py` régénère les seize STL analytiques et le manifeste avec la bibliothèque standard Python. Les positions, les paramètres et l’attribution des facettes proviennent du générateur géométrique. Ils ne dépendent d’aucune reconnaissance FabSTEP ni construction OCCT.

## Géométrie

Le bloc nominal est centré, de dimensions 24 × 20 × 18 mm. Le petit perçage de rayon `r = 3 mm` suit Z et le grand de rayon `R = 6,5 mm` suit X. Dans le repère local, leurs branches d’intersection sont

```text
Γ±(t) = (r cos t, r sin t, ±√(R² − r² sin² t))
```

Les bandes du petit perçage réutilisent ces points sur leurs bords. Chaque rangée du grand cylindre traverse une section `(y,z)` et s’arrête aux mêmes sommets de couture. Les ouvertures s’amenuisent jusqu’à `x = 0` à leurs extrémités en `y`. Les rangées hors des ouvertures partagent leur sommet à `x = 0`. Les quatre plans d’entrée reprennent les sommets exacts de leurs anneaux. Les parties du bloc se rejoignent par ses douze arêtes droites.

Le générateur relit les STL, puis vérifie la connexité, les incidences d’arêtes orientées, les liens des sommets et le nombre de contours de chaque face source. Chacune des deux coutures et chacun des quatre anneaux d’entrée forment un cycle. Les sources fermées ont une caractéristique d’Euler de −4. Par domaine, elle vaut −2 pour la grande paroi (quatre contours), 0 pour les deux petites parois et les quatre plans percés (deux contours chacun), et 1 pour les deux plans pleins. Le manifeste indique les plages contiguës de triangles par domaine, les incidences, les longueurs des cycles et les empreintes SHA-256.

## Cas

| STL | Variation |
| --- | --- |
| `f_nominal` | Bloc nominal, 64 intervalles par couture, trois subdivisions axiales. |
| `f_scaled` | Échelle 1,5 et résolution circulaire accrue. |
| `f_rotated` | Rotation oblique et translation, STL ASCII à 17 chiffres significatifs pour conserver les plans. |
| `f_mirror` | Réflexion et translation, orientation des facettes inversée pour conserver l’extérieur. |
| `f_retessellated` | Même solide que le nominal avec 88 intervalles par couture et cinq subdivisions axiales. |
| `f_ratio` | Rayons 3 et 5,5 mm. Le rapport de 0,54545… entre dans le domaine de R-091. Le STL historique est identique octet pour octet. |
| `f_ratio_mid` | Rayons 3 et 5 mm, rapport 0,60. Pose inclinée et 80 intervalles par petite bouche. |
| `f_ratio_strip` | Même rapport de 0,60, dans le repère du bloc. Une seule bande axiale relie directement la bouche à la jonction sur chaque petite paroi. |
| `f_ratio_near` | Rayons 3 et 4,51 mm, rapport 0,665188… ; pièce réfléchie et 96 intervalles. |
| `f_ratio_boundary`, `f_ratio_above` | Rayons 3 et 4,50 mm au seuil de 2/3, puis 3 et 4,49 mm au-delà ; la prise en charge analytique doit être refusée. |
| `f_off_center` | Axe du petit trou décalé de 1,125 mm en X et de 0,625 mm en Y. Les deux vides se rejoignent et les deux axes sont gauches. |
| `f_off_center_rotated` | Décalages opposés, rotation et translation du bloc, 80 intervalles par couture. |
| `f_offset_clearance` | La distance transverse de 1,4 mm sort du domaine prouvé pour ces rayons. |
| `f_fold` | Sommet intérieur de la petite paroi positive déplacé sur son propre cylindre au-delà de la colonne angulaire suivante. Des triangles inversent leur orientation géométrique, tandis que les incidences et les deux coutures demeurent. |
| `f_missing_mouth` | Plan d’entrée Z positif absent. La relecture trouve deux contours ouverts, de 4 et 64 arêtes. |

Les poses inclinées du générateur analytique emploient le STL ASCII. Les témoins FreeCAD décrits ci-dessous sont binaires. Les cas de domaine et de pli restent fermés au sens topologique. Le pli introduit un recouvrement local et sert à vérifier un refus géométrique.

## Références

Le volume idéal est celui du bloc, moins les deux cylindres traversants entiers, plus leur intersection. La section d’intersection à ordonnée `y` est le rectangle `[-√(r²−y²), √(r²−y²)] × [-√(R²−y²), √(R²−y²)]` dans le plan XZ. Par substitution `y = r sin t`, son volume devient

```text
8r² ∫[0,π/2] cos²(t) √(R² − r² sin²(t)) dt
```

La formule est évaluée en double précision par Simpson composite à 1 024 et 2 048 pas. L’écart des deux évaluations est inférieur à 10⁻⁹ mm³ sur les positifs. Le manifeste donne cet écart numérique, le volume d’intersection et le volume du solide. Le volume nominal calculé est **5 302,986330628418 mm³**.

Les bornes mondiales sont calculées à partir des huit coins placés. Douze sondes par source positive contrôlent le vide commun, les quatre branches, la matière extérieure et les deux côtés des coutures. Le générateur compare les classifications analytiques aux nombres d’enroulement calculés sur les STL relus.

## Régression de priorité

`../historical_four_blind_drills.stl` conserve les mêmes quatre entrées mais termine chaque perçage séparément par une pointe conique. Ce n’est pas un positif F : les quatre cavités sont disjointes. `python3 scripts/generate-blind-drill-fixture.py` régénère ses 1 044 facettes et vérifie fermeture, orientation, connexité, χ=2 et sondes de matière. Les anciens parcours Primitives et Surfaces doivent continuer à reconstruire six plans, quatre cylindres et quatre cônes ; la nouvelle hypothèse F ne doit pas les intercepter.


## Témoins FreeCAD indépendants

`freecad_mid_coarse.stl` vient d’un bloc de 24 × 20 × 18 mm percé par deux soustractions booléennes dans FreeCAD 1.1.1. La référence analytique a des rayons de 3 et 5 mm sur Z et X. Le mailleur triangule le STEP relu avec une déflexion linéaire de 0,025 mm et une déflexion angulaire de 0,20 rad. Les 884 triangles du STL ont leurs sommets cylindriques sur les bouches et les jonctions.

`freecad_mid_rotated.stl` utilise la même pièce, orientée et déplacée comme `f_ratio_mid` **avant le maillage**. Les coordonnées du STL binaire arrondies en simple précision peuvent fragmenter les plans percés lors de la reconstruction. Les deux fichiers ci-dessous sont reproductibles avec FreeCAD 1.1.1.

```sh
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/validate-cross-drill-ratios-freecad.py artifacts/cross-drill-reference --mesh
cmp fixtures/cross_drill/freecad_mid_coarse.stl artifacts/cross-drill-reference/freecad_plain_coarse.stl
cmp fixtures/cross_drill/freecad_mid_rotated.stl artifacts/cross-drill-reference/freecad_rotated_coarse.stl
```

Le script produit une seconde tessellation à 0,10 rad dans chaque pose. Il teste également une rotation de 24° autour de l’axe (1, 2, 3), avec le centre du bloc placé en (3, −2, 4) mm. Pour cette pose, FreeCAD applique le placement inverse au STEP relu afin de vérifier les dimensions, les axes et les sondes dans le repère du bloc. Le corpus comporte aussi deux maillages dans le repère du bloc où chaque outil de perçage comprend trois sections coaxiales. Ces sections ajoutent des cercles de subdivision sur les parois. L’option `--cli target/debug/fabstep` lance la conversion Auto aux réglages par défaut puis vérifie le STEP réimporté dans FreeCAD. Les contrôles couvrent les deux boucles communes, les supports cylindriques, les douze sondes matière/vide, les dimensions, le volume et les écarts échantillonnés des coutures, dans le repère de chaque pièce.

En mode Auto, les STL FreeCAD des perçages sans subdivision (884 et 1 766 triangles) donnent chacun un solide à six plans et trois cylindres. Les bornes continues calculées sont respectivement de 0,021398 et 0,003347 mm, arrondies par excès. Avec subdivisions (1 388 et 2 774 triangles), les bornes sont de 0,021439 et 0,003366 mm. Les 48 sondes de matière et de vide donnent le résultat attendu après réimport des quatre STEP.

Sur les STL binaires inclinés à 884 et 1 766 triangles, les bornes continues obtenues sont de 0,021399 et 0,003349 mm, arrondies par excès. Les deux solides réimportés dans FreeCAD comportent six plans et trois faces cylindriques. Les 24 sondes matière/vide passent et l’écart de volume par rapport à l’intégrale des sections est inférieur à 0,00010 mm³ sur chaque solide.

## Perçages décentrés

Le grand trou reste centré et aligné sur X. Le petit trou traverse le bloc selon Z à la position locale (a, d). Ses deux branches ont pour coordonnées `(a + r cos t, d + r sin t, ±√(R² − (d + r sin t)²))`. Le moteur vérifie `|d| ≤ r`, le dégagement des quatre entrées et la condition `R > 1,5 (r+|d|)` avec marge sur les rayons et les deux axes. Le certificat après réimport encadre continûment la courbe d’intersection et ses deux traces sur les cylindres. Le volume de référence provient d’une quadrature des sections du dessin.

Sur le témoin décentré, le volume de référence vaut 5 301,123749 mm³. La commande de qualification FreeCAD ci-dessus vérifie également les variantes de décalage positif et négatif et les deux maillages indépendants du premier témoin. Le fichier à essayer avec les réglages initiaux est `freecad_offset_coarse.stl` (886 triangles). Chaque STEP qualifié contient six plans et trois parois cylindriques.
