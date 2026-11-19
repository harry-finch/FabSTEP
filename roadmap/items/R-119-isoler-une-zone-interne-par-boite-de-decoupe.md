---
id: R-119
titre: "Isoler une zone interne par boîte de découpe visuelle 3D"
resume: "Ajouter une boîte de découpe visuelle manipulable dans la vue 3D (Source et Résultat STEP) pour inspecter, sélectionner et corriger des cavités internes sans modifier le solide ni l'export."
statut: a-tester
jalon: atelier
cree: 2026-11-19
maj: 2026-11-19
livree:
---


## But

Sur les boîtiers creux, poches profondes et pièces à cavités internes, les parois extérieures masquent les faces que l’utilisateur veut inspecter, sélectionner ou corriger en guidage. Ajouter une boîte de découpe visuelle 3D (*Box View*) dans la vue 3D permet d’isoler temporairement un volume d’intérêt pour voir et sélectionner librement l’intérieur, sans couper la géométrie réelle ni invalider un STEP déjà validé.

## Inclus

- Commande d’activation/désactivation de la **boîte de découpe visuelle** dans l’atelier (français et anglais, jetons CSS sombre/clair, contrôles 28 px), utilisable en vue **Source** comme en vue **Résultat STEP**, y compris sur un gros STL en aperçu brut.
- Six faces de découpe alignées sur X/Y/Z, initialisées à la boîte englobante de la pièce affichée, manipulables par poignées dans la vue 3D et par réglages numériques de bornes min/max avec bouton de réinitialisation.
- Filtrage cohérent du picking et de la peinture de faces : les triangles ou faces masqués hors de la boîte de découpe ne peuvent pas être sélectionnés ni peints à travers la coupe ; l’orbite caméra reste suspendue pendant le glissement d’une poignée et `Échap` annule le geste sans vider la sélection.
- Opération strictement visuelle : activer, déplacer ou réinitialiser la boîte ne relance aucun calcul CAO, n’altère jamais le solide BRep et n’invalide jamais un résultat STEP ou DXF exportable.
- Réinitialisation automatique de la boîte lors du chargement d’une nouvelle source ou d’un changement d’unité STL.

## Hors périmètre

- Découpe géométrique réelle du solide STEP ou export d’une sous-partie découpée (c’est le rôle distinct des coupes DXF `R-053`/`R-054`).
- Boîte de découpe orientée librement (rotation arbitraire de la boîte) dans cette première tranche : les six plans restent alignés sur les axes du repère.
- Modification du moteur Rust (`fabstep-core` / `fabstep-cad`) ou ajout d’une nouvelle dépendance Three.js.

## Dépendances

- Aucune dépendance bloquante ; cohabite avec l’outil de coupe plane (`R-053` / `R-054`) sans conflit de manipulateurs dans `Viewport.svelte` (un seul manipulateur 3D actif à la fois).

## Critères d'acceptation

- L’activation de la boîte de découpe masque instantanément la géométrie hors des six bornes (maillage et arêtes de contour) en vue Source et en vue STEP, dans les thèmes sombre et clair, sans invalider l’état exportable courant.
- Un clic ou un glissement de peinture de faces atteint la première face visible **à l’intérieur** de la boîte et ignore toute face coupée devant elle.
- Le glissement d’une poignée ne fait jamais passer `min >= max` (borne minimale strictement inférieure à la borne maximale) ; `Échap` annule le déplacement en cours sans effacer la sélection active.
- Charger un nouveau fichier STL/3MF ou changer d’unité réinitialise proprement la boîte aux dimensions de la nouvelle pièce ; aucune ressource WebGL n’est divulguée au démontage.
- `npm --prefix ui run test`, `npm --prefix ui run check`, `npm --prefix ui run build` et `sh scripts/check.sh` passent sans erreur ni avertissement.

## Vérifications de l'utilisateur

- Ouvrir un boîtier creux ou un bac percé dans FabSTEP, isoler une cavité interne avec la boîte de découpe, sélectionner puis peindre une face intérieure et vérifier que la sélection ne traverse pas la paroi masquée.
- Vérifier le contraste des poignées et du cadre de boîte en thèmes sombre et clair, en français et en anglais.
