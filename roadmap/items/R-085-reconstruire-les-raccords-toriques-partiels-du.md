---
id: R-085
titre: "Reconstruire les raccords toriques partiels du pourtour de Flansch"
resume: "Sur Flansch, le laboratoire réimporte 2 083 faces et retrouve leurs deux contours sur le STL. Raccorder les 80 126 triangles restants, puis valider le solide dans FabSTEP."
statut: en-cours
jalon: post-v1
cree: 2026-11-18
maj: 2026-11-19
livree:
---

## But

Reconstruire les congés toriques partiels d’une bride lorsque leur contour et les faces qui le partagent sont démontrés.

## Inclus

Identifier les limites dans les deux bandes de Flansch. Introduire un secteur à quatre arcs partagés, accepté explicitement le 18 novembre 2026. Qualifier une première configuration fermée à deux plans radiaux, ses variantes placées et ses contre-exemples. Étendre ensuite le raccordement aux congés cylindriques tangents réellement présents sur Flansch.

## Hors périmètre

Approximation de contours irréguliers, hausse des plafonds actuels, export de coque ouverte ou de partie du document.

## Dépendances

Diagnostic R-080 et contrat ADR-0040. Les autres raccords du pourtour dépendent de la qualification des voisins tangents.

## Critères d'acceptation

Attribution exhaustive, bornes dans les deux sens, limites partagées et contrôle natif après STEP. La première configuration doit conserver sept faces sélectionnables sur le corpus public en Auto. Pour étendre le raccordement entre une paroi courbe et les facettes voisines, qualifier aussi un solide indépendant de Flansch en mode Auto. Vérifier les faces reconnues, le partage du bord, le volume après réimport et le comportement sur une limite déformée. Pour Flansch, vérifier les voisins et leurs autres bords, assembler le corps complet, rejouer le helper et consigner le premier refus ou la validation du solide.

## Vérifications de l'utilisateur

Essayer le secteur public dans FabSTEP et vérifier le solide exporté dans Fusion. La qualification du pourtour réel reste ouverte dans ce jalon.

## État vérifié

Le secteur public à plans radiaux est intégré dans Auto et Surfaces. Les essais FabSTEP et Fusion A-084 et A-085 restent ouverts. Le helper Auto convertit aussi la rampe dense indépendante de 194 694 triangles en un solide de 659 faces (V-1250).

Sur le STL Flansch original, 43 surfaces toriques couvrent 493 542 des 540 170 triangles. Le solide construit directement en laboratoire présente 46 671 faces. Ses arêtes passent le contrôle géométrique exact après transfert. Le témoin fermé est tessellé à 0,005 mm en 98 576 triangles ; les écarts maximaux échantillonnés sont de 0,011553 et 0,010018 mm (V-1252/V-1253). Le premier refus d’auto-intersection a été reproduit puis corrigé sur les six raccords concernés. Les 14 882 raccords passent le contrôle individuel. Le rejeu du paquet corrigé atteint la limite de trente minutes pendant le contrôle complet (`worker.timeout`, V-1258). Aucun STEP de Flansch n’est exportable. Le lot public est à tester. Le volet Flansch reste ouvert sur le coût du contrôle entre faces, avec les plafonds actuels.
