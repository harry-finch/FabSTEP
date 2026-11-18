---
id: R-085
titre: "Reconstruire les raccords toriques partiels du pourtour de Flansch"
resume: "Le laboratoire réimporte 34 faces de Flansch et retrouve 79 879 faces sur la partition du corps entier. Raccorder les 16 contours libres aux surfaces conservées, puis vérifier le solide complet et son export depuis FabSTEP."
statut: en-cours
jalon: post-v1
cree: 2026-11-18
maj: 2026-11-18
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

Attribution exhaustive, bornes dans les deux sens, limites partagées et contrôle natif après STEP. La première configuration doit conserver sept faces sélectionnables sur le corpus public en Auto. Pour Flansch, vérifier les voisins et leurs autres bords, assembler le corps complet, rejouer le helper et consigner le premier refus ou la validation du solide.

## Vérifications de l'utilisateur

Essayer le secteur public dans FabSTEP et vérifier le solide exporté dans Fusion. La qualification du pourtour réel reste ouverte dans ce jalon.

## État vérifié

Le domaine à plans radiaux est intégré dans Auto et Surfaces. Les essais A-084 et A-085 restent ouverts. Sur Flansch, le laboratoire réimporte une coque connexe de 34 faces comprenant le dessous de la bride, les raccords haut et bas, cinq entrées de perçage et le flanc à dix découpes. Les 34 faces couvrent 164 392 triangles. En les substituant à leurs propriétaires actuels, la partition du document entier compte 79 879 faces. Le helper de l’application refuse encore le fichier à 231 608 faces.

Il reste à raccorder cinq anneaux de perçage, dix contours du marquage et le contour principal de 20 arêtes aux surfaces conservées, puis à valider le solide entier. Les 692 arêtes source de ce dernier contour sont attribuées à leurs voisines. La prochaine difficulté géométrique porte sur les trois flancs cylindriques longs à bord supérieur irrégulier. Les preuves et commandes de reprise figurent dans V-1209.
