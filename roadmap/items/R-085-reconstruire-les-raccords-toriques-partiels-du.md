---
id: R-085
titre: "Reconstruire les raccords toriques partiels du pourtour de Flansch"
resume: "Sur Flansch, le laboratoire réimporte 2 083 faces et retrouve leurs deux contours sur le STL. Raccorder les 80 126 triangles restants, puis valider le solide dans FabSTEP."
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

Attribution exhaustive, bornes dans les deux sens, limites partagées et contrôle natif après STEP. La première configuration doit conserver sept faces sélectionnables sur le corpus public en Auto. Pour étendre le raccordement entre une paroi courbe et les facettes voisines, qualifier aussi un solide indépendant de Flansch en mode Auto. Vérifier les faces reconnues, le partage du bord, le volume après réimport et le comportement sur une limite déformée. Pour Flansch, vérifier les voisins et leurs autres bords, assembler le corps complet, rejouer le helper et consigner le premier refus ou la validation du solide.

## Vérifications de l'utilisateur

Essayer le secteur public dans FabSTEP et vérifier le solide exporté dans Fusion. La qualification du pourtour réel reste ouverte dans ce jalon.

## État vérifié

Le domaine à plans radiaux est intégré dans Auto et Surfaces. Les essais A-084 et A-085 restent ouverts. Sur Flansch, le laboratoire réimporte une coque de 2 083 faces comprenant les trois flancs longs, les cinq chaînes de perçage, les épaulements et les dix marquages en creux. Elle inclut aussi 267 facettes raccordées au bord des cylindres longs et couvre 460 044 des 540 170 triangles du fichier.

Trois nappes et trois triangles isolés, soit 80 126 triangles, restent à raccorder. Leurs 77 396 propriétaires porteraient le total à 79 479 faces. La suite porte sur les limites courbes partagées avec ces facettes, le solide entier, sa tessellation et la conversion dans l’application. Le helper actuel refuse encore Flansch à 231 608 faces. Reproduction et mesures dans V-1210.
