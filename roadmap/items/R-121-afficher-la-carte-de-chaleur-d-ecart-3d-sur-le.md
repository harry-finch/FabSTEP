---
id: R-121
titre: "Afficher la carte de chaleur d'écart 3D sur le résultat STEP"
resume: "Superposer dans la vue Résultat STEP une coloration par écart échantillonné au maillage source avec légende en millimètres, calculée à la demande en lecture seule sans affecter l'export."
statut: livree
jalon: atelier
cree: 2026-11-19
maj: 2026-11-20
livree: 2026-11-20
---


## But

Aujourd’hui, l’audit de fidélité n’affiche que deux valeurs numériques globales (les maxima échantillonnés source→résultat et résultat→source). Lorsque l’écart approche le budget ou qu’une zone intéresse l’utilisateur, impossible de voir immédiatement *où* se situe l’écart maximal sur la pièce. Afficher une **carte de chaleur d’écart 3D** directement sur le résultat STEP permet de localiser d’un coup d’œil les zones exactes et les zones proches du seuil, sans modifier le solide ni l’export.

## Inclus

- Mode d’affichage activable à la demande dans la vue **Résultat STEP** permettant de basculer entre **Couleurs par face** (mode par défaut inchangé) et **Écart au maillage** (français et anglais, jetons CSS sombre et clair, contrôles 28 px).
- Calcul natif en **lecture seule** hors fil UI (`fabstep-verify` / `fabstep-app` / commande bureau dédiée) mesurant, pour chaque sommet ou triangle de la tessellation affichée du STEP validé, sa distance exacte au maillage source soudé (même arbre BVH et mêmes limites que `fabstep-verify`, budget borné à 50 millions d’opérations, plafonds ordinaires de 100 000 triangles / 300 000 sommets).
- Coloration par sommet dans `Viewport.svelte` selon une palette lisible définie par jetons CSS (en sombre et en clair), étalonnée sur le budget d’écart actif (`max_deviation_mm`), accompagnée d’une **légende compacte en millimètres** indiquant le minimum, le maximum mesuré sur la tessellation affichée et le seuil de référence, avec mention explicite qu’il s’agit d’écarts échantillonnés non signés.
- Affichage, dans l’inspecteur de face sélectionnée, de l’écart maximal local mesuré sur la face STEP choisie ; maintien de la visibilité des contours de faces et du surlignage de sélection.
- Gardes de concurrence stricts (révision, identité de source, époque et identité du `DocumentStep`) : un calcul de carte de chaleur en échec, annulé ou obsolète n’invalide jamais un STEP exportable courant et revient proprement à l’affichage par face.

## Hors périmètre

- Carte d’écart **signée** (+/− matière en excès ou en manque) dans cette première tranche : l’écart affiché est une distance non signée (≥ 0 mm), cohérente avec l’audit existant de `fabstep-verify`.
- Affichage de la carte de chaleur sur le mode gros STL isolé (`large-stl`, ADR-0034) ou sur un résultat refusé sans STEP validé.
- Modification des critères d’acceptation, des budgets ou des valeurs rapportées par l’audit de fidélité officiel de `fabstep-verify`.
- Export de maillage coloré (PLY/OBJ/3MF coloré) ou capture d’image automatisée.

## Dépendances

- S’appuie sur la tessellation d’affichage du résultat STEP (`result_faces`, `R-044` / `I1b`) et sur le moteur de distance BVH de `fabstep-verify`. Aucune nouvelle dépendance tierce.

## Critères d'acceptation

- Sur une pièce analytique validée (ex. `fixtures/m2b_boss_blind.stl` ou `fixtures/m5_toroidal_fillet.stl`), activer **Écart au maillage** colore la vue STEP selon les distances exactes calculées par `fabstep-verify` et affiche une légende cohérente avec le maximum mesuré et le budget `max_deviation_mm`.
- Le maximum global de la carte concorde avec le calcul exact de référence sur les mêmes sommets (test unitaire vérifiant l’accord bit à bit avec un calcul exhaustif sur un témoin synthétique).
- La sélection de faces STEP, les contours de faces, la boîte de découpe visuelle (`R-119`) et l’export STEP/DXF restent pleinement fonctionnels ; un refus de budget de la carte de chaleur laisse le STEP courant exportable.
- Changer de fichier, d’options ou relancer une conversion annule et ignore toute carte de chaleur obsolète en vol.
- `sh scripts/check.sh` passe sans erreur ni avertissement.

## Vérifications de l'utilisateur

- Ouvrir une pièce courbe (ex. congé torique ou pièce mixte) dans FabSTEP, passer en vue Résultat STEP, activer **Écart au maillage**, vérifier la lisibilité de la palette et de la légende en thèmes sombre et clair, puis sélectionner une face pour lire son écart local.
- Vérifier que basculer entre **Couleurs par face** et **Écart au maillage** reste réactif et n’altère jamais la disponibilité de **Exporter STEP**.
