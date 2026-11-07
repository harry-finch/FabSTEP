# Corpus d’acceptation mécanique v1

Le corpus part de solides cotés construits avec FreeCAD Part et les maille sans utiliser FabSTEP. Le [manifeste](manifest.json) fixe les volumes nominaux, les boîtes englobantes, les surfaces et des points de contrôle dans le solide ou les cavités. Les STL gardent les coordonnées en ASCII pour isoler d’abord la reconnaissance de la quantification binaire. Les régressions déjà documentées pour les STL et 3MF réels, les placements, les rotations et la précision complètent ce corpus.

| Cas | Géométrie nominale | Dimensions mm | Volume nominal mm³ | Triangles |
| --- | --- | --- | ---: | ---: |
| 01 — Bloc de distribution | Traversant R3 suivant Z. Deux borgnes à fond plan R2 × 8 suivant X et R2,5 × 9 suivant Y. 8 plans et 3 cylindres. | 48 × 36 × 18 | 30 317,816438 | 2 764 |
| 02 — Bague à deux chanfreins | Alésage R5 et deux cônes à 45° de part et d’autre d’un même cylindre extérieur R15. Rayons extérieurs 12 → 15 puis 15 → 11. 2 plans, 2 cylindres, 2 cônes. | 30 × 30 × 17 | 9 598,612754 | 4 128 |
| 03 — Carter mixte | Traversant R3 fraisé à 45° jusqu’à R5, borgne R2 × 8 perpendiculaire, cuvette sphérique R5 de profondeur 3, bossage R8 à congé torique R6/r2. 8 plans, 3 cylindres, 1 cône, 1 sphère, 1 tore. | 50 × 40 × 24 | 32 855,662329 | 41 290 |
| 04 — Boîtier arrondi | Congés R2 sur douze arêtes et raccords sphériques R2 aux huit coins. 6 plans, 12 cylindres partiels et 8 portions sphériques. | 28 × 20 × 14 | 7 637,828852 | 28 700 |
| 05 — Alésage elliptique | Demi-axes 5 et 4. `recognition.surface_unproven` attendu en mode analytique. | 28 × 26 × 10 | 6 651,681469 | 1 176 |
| 06 — Bloc ouvert | Même maillage que le 01 avec un triangle retiré. `mesh.open` attendu et aucun STEP. | — | — | 2 763 |

Les volumes nominaux sont obtenus par soustraction des cylindres pour 01, par sommes de cylindres et troncs de cône pour 02, et par les mêmes volumes complétés d’une intégrale du profil du congé et du volume de calotte pour 03. Le volume 04 additionne le pavé intérieur (24 × 16 × 10), les six plaques d’épaisseur 2, les douze quarts de cylindre et les huit octants de sphère. Le volume 05 soustrait l’aire elliptique `π × 5 × 4` sur la hauteur 10.

## Mesures et seuils

- Les références 01–05 ont été réimportées avec FreeCAD 1.1.1. Chacune contient un solide fermé et valide. Les 2 raccords du congé torique de 03 et les 48 raccords entre surfaces de 04 sont tangents aux trois points échantillonnés par arête. Le volume STEP mesuré du témoin elliptique diffère du volume nominal de 0,000523 mm³, dans le seuil spécifique de référence de 0,001 mm³.
- Un STEP candidat analytique doit contenir un solide fermé et valide, les surfaces indiquées avec leurs rayons à 0,01 mm près et leurs demi-angles à 0,01° près, et conserver les points de matière et de vide du manifeste. Chaque extrémité de la boîte est comparée à 0,05 mm et le volume à `max(0,1 mm³, volume × 10⁻⁵)`. Les raccords indiqués doivent rester tangents à 0,01° près aux trois échantillons par arête.
- La CLI doit accepter les positifs au budget analytique initial de 0,05 mm, puis signaler la borne composée et les deux audits de distance échantillonnés. Le réimport FreeCAD vérifie séparément la géométrie. Un refus analytique ou une conversion qui laisse facettisées les surfaces attendues échoue à cette acceptation. La fidélité mesurée ne constitue pas une borne globale de Hausdorff.
- Le témoin 05 vérifie que le mode analytique ne transforme pas une ellipse en cercle. Un export explicitement demandé en mode Plans reste admissible sous les critères du mode facettisé. Le témoin 06 doit rester inexploitable dans les deux modes. Tester les refus dans un dossier de sortie vide et contrôler les codes avec les rapports JSON. Réutiliser les tests de non-régression pour l’invalidation de l’export courant et le maintien de l’aperçu source.

## Reproduire

Depuis la racine du dépôt, avec le Python fourni par FreeCAD sur ce Mac.

```sh
export PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib
FREECAD_PYTHON=/Applications/FreeCAD.app/Contents/Resources/bin/python
"$FREECAD_PYTHON" fixtures/complex_v1/freecad_corpus.py
```

Les références analytiques STEP, les STL et le rapport `measurements.json` sont générés dans `artifacts/complex-v1/reference/`. Ce dossier est exclu de Git. La tessellation de chaque STL utilise une déflexion de 0,01 mm. Les volumes et géométries attendus proviennent du manifeste. La taille de maillage mesurée dépend de la version de FreeCAD et d’OCCT.

Pour mesurer les STEP obtenus avec FabSTEP, utiliser un dossier candidat dédié. Les chemins de sortie doivent garder les identifiants du manifeste. Choisir le binaire CLI qui correspond au build à vérifier.

```sh
CORPUS=artifacts/complex-v1
CANDIDATES=$(mktemp -d "$CORPUS/candidates.XXXXXX")
FABSTEP_CLI=target/debug/fabstep
for input in "$CORPUS"/reference/*.stl; do
  case_id=$(basename "$input" .stl)
  "$FABSTEP_CLI" convert "$input" -o "$CANDIDATES/$case_id.step" \
    --mode surfaces --unit mm --json > "$CANDIDATES/$case_id.json" \
    2> "$CANDIDATES/$case_id.stderr"
  printf '%s %s\n' "$case_id" "$?"
done
for case_id in 05-elliptic-bore 06-open-manifold; do
  "$FABSTEP_CLI" convert "$CORPUS/reference/$case_id.stl" -o "$CANDIDATES/$case_id-planar.step" \
    --mode planar --unit mm --json > "$CANDIDATES/$case_id-planar.json" \
    2> "$CANDIDATES/$case_id-planar.stderr"
  printf '%s-planar %s\n' "$case_id" "$?"
done
"$FREECAD_PYTHON" fixtures/complex_v1/freecad_corpus.py --candidates "$CANDIDATES"
```

Le script FreeCAD mesure les sorties positives et vérifie les réglages, la borne composée et la présence des deux audits CLI. Pour les refus, il vérifie l’absence de STEP et le code d’erreur dans le rapport JSON d’erreur. Le STEP facettisé du 05 fait l’objet d’un réimport indépendant.

## Périmètre mécanique à vérifier avant v1

Les quatre positifs couvrent les perçages sur plusieurs axes, les trous traversants et borgnes, les chanfreins circulaires, les transitions entre cylindres et plusieurs cônes, les cuvettes et les raccords de révolution, puis les congés rectilignes et leurs coins sphériques. Les deux négatifs imposent un refus reproductible lorsque la surface n’a pas de preuve analytique dans le modèle ou que la coque est ouverte. Pour annoncer ce périmètre, exécuter les quatre cas positifs jusqu’au réimport indépendant, les deux refus, les régressions sur pièces utilisateur et les contrôles natifs import/export sur le paquet distribué.

Les intersections de perçages, les raccords à rayon variable, les ellipses et les surfaces libres nécessitent des preuves et des témoins dédiés avant d’être annoncés en mode analytique. Les limites d’entrée et de calcul restent applicables à tous les cas.

## État mesuré en 0.10.0-alpha.1

Le binaire CLI release macOS ARM64 existant a été exécuté aux seuils initiaux. Les seuls STEP exportés par cette campagne, le bloc 01 et l’ellipse facettisée 05, ont tous deux été réimportés dans FreeCAD 1.1.1. [Exécutions CLI](../../artifacts/complex-v1/baseline-0.10.0-alpha.1/runs.json) · [Réimports et refus](../../artifacts/complex-v1/baseline-0.10.0-alpha.1/measurements.json) · [Références](../../artifacts/complex-v1/reference/measurements.json)

| Cas | Résultat |
| --- | --- |
| 01 | **Accepté.** Le STEP contient les 11 faces attendues et conserve les trois rayons, les fonds borgnes et le traversant. Borne composée 0,0002833 mm. Maxima échantillonnés 0,0024966 mm source → STEP et 0,0024964 mm STEP → source. |
| 02 | **Écart.** `recognition.surface_unproven` indique plusieurs portions coniques ou toriques dans une même chaîne. Aucun STEP. |
| 03 | **Écart.** `check` reconnaît trois cylindres, un cône, une sphère et un tore, avec borne composée 0,0020784 mm. `convert` refuse après 26,34 s de calcul sur ce poste avec `validation.distance_budget`. Aucun STEP et aucun audit complet pour le maillage à 41 290 triangles. |
| 04 | **Écart.** `recognition.surface_unproven` signale l’échec de la tentative terminale de sphère entière sur le boîtier. Aucun STEP. Les raccords cylindre–plan le long d’arêtes et cylindre–sphère en coin restent à prouver. |
| 05 | **Refus attendu.** `recognition.surface_unproven` en Surfaces, avec un diagnostic de topologie de la tentative de sphère. En Plans, FreeCAD valide un solide de 296 faces planes et 6 651,733526 mm³, avec les dimensions et les points matière/vide attendus. |
| 06 | **Refus attendu.** `mesh.open` en Plans et en Surfaces, sans STEP. |

L’acceptation du corpus exige encore les sorties analytiques 02, 03 et 04. Le cas 03 sépare une reconnaissance réussie d’un coût de vérification excessif. Une sonde produite à 0,02 mm sur la même référence FreeCAD compte 13 386 triangles et atteint aussi `validation.distance_budget` (7,30 s). Le maillage nominal reste à 0,01 mm. Les cas 02 et 04 exigent des preuves de raccords supplémentaires.

## Résolution pour la candidate v1

Les quatre positifs sont maintenant reconstruits analytiquement et réimportés dans FreeCAD. Le double chanfrein est accepté en mode Surfaces avec deux cônes opposés séparés par une portée cylindrique résolue. L’audit du carter conserve exactement ses échantillons et distances mais passe sous le budget de travail inchangé. Le boîtier est reconnu comme un corps convexe entièrement arrondi, avec ses 26 faces et 48 raccords tangents.

La [campagne 01–03 et négatifs](../../artifacts/complex-v1/audit-optimization/cli-replay/measurements.json) conserve son échec historique du 04 ; le [réimport final du 04](../../artifacts/m6a/final/cas04-freecad.json) constate sa résolution. Les [régressions privées](../../artifacts/complex-v1/private-regression/freecad-summary.json) passent également. Les contrôles du paquet et du parcours natif de la candidate sont consignés séparément dans [VALIDATION](../../docs/VALIDATION.md), pour ne pas les confondre avec l’acceptation géométrique.
