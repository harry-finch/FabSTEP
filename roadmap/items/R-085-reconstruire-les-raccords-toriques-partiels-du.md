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

Le domaine à plans radiaux est intégré dans Auto et Surfaces. Les essais A-084 et A-085 restent ouverts. Sur Flansch, le laboratoire réimporte maintenant un solide fermé de 13 802 faces. Les 540 170 triangles source ont un propriétaire. La tessellation OCCT à 0,005 mm atteint 215 486 triangles. Le helper de production conserve son refus. V-1237 décrit le rejeu du solide, le témoin indépendant et les quatre contrôles de refus. La suite porte sur le maillage à bords communs et la construction directe du candidat dans le helper.
