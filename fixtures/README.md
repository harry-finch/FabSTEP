# Corpus synthétique

Les fichiers `m2_*.stl` sont produits par `python3 fixtures/generate_m2.py`, sans dépendance ni appel au convertisseur. Le générateur et les données sont distribués sous GPL-3.0-or-later, comme le projet. Les STL ne contiennent pas d'unité ; les coordonnées sont à interpréter en millimètres.

Le manifeste `m2_manifest.json` donne les volumes **analytiques nominaux**, les rayons, le nombre de faces cibles et la flèche maximale de corde de la tessellation source. Les volumes des STL polygonaux sont légèrement différents des volumes nominaux. La plaque emploie 16 points par côté, y compris les coins ; les angles du trou en résultent et ne sont pas uniformes. Les points supplémentaires du bord rectangulaire sont collinéaires.

- Cylindre R10 × 20 : versions standard, quatre intervalles axiaux, et même version tournée/translatée.
- Tube traversant R10 / R5 × 20 ; plaque 30 × 20 × 5 avec trou traversant R3.
- Négatifs : tronc de cône R10 → R8 et fût à constriction centrale, avec bouchons circulaires identiques. Une reconnaissance sur les seuls bouchons est insuffisante.

Ces attentes ne prouvent pas une reconnaissance : vérifier la totalité de la paroi, la validité du solide et la distance au maillage. Les volumes, rayons et nombres de faces sont des références, pas des résultats de tests. Pour contrôler un STEP indépendamment, `scripts/validate-freecad.py` accepte `--faces`, `--planes`, `--cylinders`, `--volume` et `--dimensions` et rapporte les rayons et axes des cylindres FreeCAD.

## Corpus M2b — pièces mixtes

`python3 fixtures/generate_m2b.py` produit, sans dépendance externe ni code FabSTEP, les `m2b_*.stl` et leur manifeste. Les parois du bloc sont découpées en cellules rectangulaires, reliées par des sommets communs ; les contours circulaires sont triangulés radialement. Les cas positifs comprennent un trou borgne à fond plat (profondeur 6, fond à Z = 4), un bloc à deux trous séparés (borgne et traversant), un bossage, et un bossage associé à un trou borgne. Des versions subdivisées, tournées et translatées vérifient la cohérence sur toute la paroi. Les dimensions et volumes analytiques nominaux, rayons, profondeurs et nombres de faces attendus sont dans le manifeste ; les STL eux-mêmes ont des volumes polygonaux différents.

Les cas négatifs comportent un trou borgne conique, un fond conique non plan, un bossage avec saillie intermédiaire et un trou à huit sommets seulement. Ils ne doivent pas être interprétés comme les primitives régulières correspondantes. La limite d'échantillonnage indique une ambiguïté de reconnaissance, pas une erreur topologique. Les cas tournés indiquent l'axe et la translation ; la flèche maximale est calculée à partir des angles d'échantillonnage, qui ne sont pas uniformes.

`generate_freecad_m2b.py` ajoute une référence issue du modeleur FreeCAD Part :
`m2b_orthogonal_freecad.stl`, deux trous borgnes perpendiculaires et séparés dans
un bloc 30 × 20 × 10. Rayons 3 et 2 mm, profondeurs 6 et 5 mm ; volume analytique
6000 − 74π mm³. La source tessellisée contient 3 364 triangles sur le poste de
référence, l'attente analytique est 10 faces dont deux cylindres. Le manifeste
séparé enregistre version du générateur, paramètres et unités. Ce STL est généré
par un autre modeleur ; il ne constitue pas encore un fichier utilisateur réel.

## Corpus M2c — arcs ouverts

`python3 fixtures/generate_m2c.py` crée indépendamment des prismes à encoche ouverte : un arc relie deux bords droits, sans cercle ni fond ajouté. Les cas comprennent un arc seul, une version à quatre intervalles axiaux, une rotation/translation et une pièce avec trou traversant seul ou associé à un bossage. Les variantes négatives déforment la paroi en cône, introduisent une saillie, déplacent progressivement les contours, ou limitent l'arc à cinq segments (six points). Tester l'échantillonnage insuffisant avec un budget de 1 mm pour le distinguer d'un refus pour flèche trop élevée.

Le manifeste contient le rayon, l'angle, la flèche maximale, les faces et le volume analytiques nominaux ; les volumes des STL polygonaux diffèrent. Un budget inférieur à la flèche doit refuser la reconstruction. Ce corpus synthétique est généré sans utiliser ni redistribuer le STL utilisateur.

## Corpus M3 — 3MF

`python3 fixtures/generate_m3.py` produit les archives `m3_*.3mf` avec la bibliothèque standard Python, sans utiliser FabSTEP. Elles suivent le schéma Core 3MF et l'empaquetage OPC (contenu, relation StartPart, modèle XML). Les volumes et boîtes placées attendus, en millimètres, sont dans `m3_manifest.json` ; aucune validation FabSTEP n'est sous-entendue par leur génération.

Les cas valides couvrent un cube, l'unité par défaut, deux instances, les composants imbriqués avec translation et rotation non commutatives, les pouces (y compris la translation), une symétrie et un mesh contenant deux composantes fermées. Les cas négatifs présentent un cycle, un indice de sommet absent, un maillage ouvert, une extension explicitement requise non prise en charge, ainsi que des corps qui se chevauchent, se touchent ou sont imbriqués (aucune fusion implicite). `m3_zip_budget.3mf` contient une partie modèle XML de plus de 70 Mio décompressés pour tester un budget de décompression ; ne pas l'extraire sans limite. Le seuil de refus effectif appartient au contrat de l'importeur et doit être documenté.

Référence de format : [3MF Core Specification, consortium 3MF](https://github.com/3MFConsortium/spec_core/blob/master/3MF%20Core%20Specification.md).

Le corpus M3 comprend aussi une transformation affine (échelle non uniforme et cisaillement) et `m3_freecad_instances.3mf`, produit avec FreeCAD 1.1.1 par `generate_freecad_m3.py`. Ce dernier contient deux objets cubiques de 1 mm, le second translaté de (3, 2, 0) mm ; il est indépendant du writer Python ZIP/XML. Les références sont dans `m3_manifest.json`.

## Corpus de précision

`python3 fixtures/generate_precision.py` extrait le maillage de la pièce synthétique `m2c_open_arc_mixed.stl` et produit trois paquets 3MF Core, plus un STL ASCII. Le témoin exact conserve les coordonnées du maillage de référence. Les variantes arrondies utilisent six chiffres significatifs, avec les mêmes triangles et sommets indexés. Le cas aberrant ajoute un déplacement de 0,002 mm à un seul sommet de la variante 3MF arrondie. Le STL ASCII contient les mêmes valeurs arrondies que le 3MF.

`precision_manifest.json` consigne les déplacements et les attentes de reconnaissance. Les valeurs analytiques nominales restent dans `m2c_manifest.json`. Le jeu permet de vérifier un refus au seuil par défaut sur les coordonnées arrondies, une conversion avec une tolérance d'ajustement choisie explicitement et des refus persistants pour un sommet aberrant ou un budget d'écart trop faible. Les essais sont détaillés dans [Validation](../docs/VALIDATION.md#v-504-et-v-505--corpus-synthétique-de-précision). Le générateur ne lit aucun des fichiers privés conservés dans `artifacts/`.


## Corpus M4a — cônes et chanfreins

`python3 fixtures/generate_m4.py` génère indépendamment huit STL positifs et deux 3MF Core positifs : cône tronqué, rotation/translation, pointe, cylindre chanfreiné, bossage sur plaque, fraisure, fond de perçage conique et chanfrein droit en mode Plans. Les 3MF couvrent un corps placé et deux instances séparées. Le manifeste `m4_manifest.json` donne unités, volumes analytiques, dimensions, axes, rayons terminaux, demi-angles et longueurs. Les volumes analytiques sont distincts de ceux des facettes source.

Sept témoins sont négatifs : anneau peu échantillonné, pointe décentrée, paroi à trois lobes, deux pentes contiguës, méridienne sphérique, pentes séparées par de très courtes bandes presque cylindriques et corps tangents. Le cas `m4_alternating_taper` conserve le contre-exemple de la revue : une seule plage conique est admise par chaîne connectée dans M4a. Un budget d’écart inférieur à la flèche doit aussi refuser la reconstruction.

## Corpus M5a — hémisphères et congés toriques

`generate_m5.py` produit un corpus synthétique indépendant du convertisseur et
des fichiers utilisateur. `m5_manifest.json` fixe les rayons, domaines, volumes,
axes et boîtes attendus. Il couvre une demi-sphère R6 avec triangulation
irrégulière, un cylindre R8 surmonté d'un congé torique R6/R2, leurs variantes
tournées, une combinaison avec cône et encoche cylindrique ouverte, ainsi que
deux instances 3MF placées séparément.

Les négatifs déforment la sphère ou le tore, réduisent l'échantillonnage,
suppriment la tangence requise ou mettent les deux corps en contact. Un budget
d'écart insuffisant constitue un refus supplémentaire. Les tests vérifient la
reconnaissance et l'export ; `scripts/validate-m5-freecad.py` contrôle les STEP
dans FreeCAD, notamment les normales aux deux jonctions du congé. Les mesures
exécutées figurent dans `docs/VALIDATION.md`. Les fichiers réels et leurs dérivés
restent exclusivement dans `artifacts/`.

La génération ne constitue pas une validation. Les conversions et réimports indépendants sont détaillés dans [VALIDATION](../docs/VALIDATION.md#m4a--cônes-et-chanfreins-circulaires-6-novembre-2026), et le contrôle FreeCAD reproductible est dans `scripts/validate-m4-freecad.py`. Aucune source privée de `artifacts/` n’est utilisée par ce générateur.


## Corpus M5b1 — calottes sphériques

`python3 fixtures/generate_m5b.py` génère indépendamment les calottes sphériques à anneau plan et pôle source, petites ou grandes, ainsi qu’une cavité concave, une variante tournée et un 3MF à deux instances dont une en miroir. `m5b_manifest.json` contient les références géométriques ; la génération seule ne valide pas le convertisseur. Les témoins négatifs couvrent notamment échantillonnage insuffisant, pôle déplacé, sphéroïde, zone sphérique sans pôle et corps tangents ; un budget insuffisant est également testé.

Cinq STEP CLI positifs, représentant six solides, ont été réimportés dans FreeCAD ; voir [validation](../docs/VALIDATION.md) et les [résultats du corpus](../artifacts/m5b1/verified/corpus-freecad.json). Les fichiers réels privés ne servent pas à générer ces fixtures.


## Corpus M5b2 — corps sphériques complets

`python3 fixtures/generate_m5c.py` produit indépendamment des icosphères et sphères UV, des variantes transformées ou mises à l’échelle et des instances 3MF placées. `m5c_manifest.json` contient les références attendues. Les témoins comprennent notamment une sphère déformée, une coque ouverte, une forme tronquée à bouchon plan, un échantillonnage faible et des contacts à la source ou après reconstruction. La génération et les valeurs du manifeste ne constituent pas une validation : les essais exécutés et leurs limites sont consignés dans [VALIDATION](../docs/VALIDATION.md). Aucune source privée n’est utilisée par ce générateur.

## Corpus M5d — tores entiers

`generate_m5d.py` produit le tore R6/r2, sa variante transformée f32 et deux instances 3MF dont un miroir. Le manifeste distingue les paramètres analytiques, les placements et les cas négatifs : tore déformé, ouvert, peu échantillonné, contacts source/résultat. `scripts/validate-m5d-freecad.py` contrôle les réimports indépendants. Les valeurs générées ne suffisent pas à valider le produit ; les résultats sont dans [VALIDATION](../docs/VALIDATION.md).

## Corpus M6a — boîtiers arrondis

`generate_m6a.py` produit une boîte convexe R2 de 28 × 20 × 14 mm, une triangulation tournée f32 et un document 3MF de deux instances séparées. `m6a_manifest.json` fixe les 6 plans, 12 quarts de cylindre et 8 octants attendus. `scripts/validate-rounded-box-freecad.py` vérifie les paramètres, 48 raccords et points matière/vide. Le [corpus mécanique indépendant](complex_v1/README.md), construit directement dans FreeCAD, complète ce jeu. Les générateurs ne lisent aucun fichier privé.

## Régressions de validation et de noms

`generate_v1_source_validation.py` crée un témoin de plaque auto-intersectée et une plaque peu profonde valide : le regroupement planaire ne doit pas masquer l’intersection source. `generate_name_origins.py` compare un nom 3MF absent, vide et un nom fourni identique à une traduction générée : seul le nom généré doit être traduit.
