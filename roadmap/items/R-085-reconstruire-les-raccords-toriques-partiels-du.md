---
id: R-085
titre: "Reconstruire les raccords toriques partiels du pourtour de Flansch"
resume: "Commencer par les deux bandes de rayon 0,5 mm identifiées dans R-080. Qualifier les limites des secteurs, la continuité avec les faces voisines et la fidélité dans les deux sens sur un corpus public indépendant. Rejouer ensuite le fichier entier et mesurer les coûts natifs."
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

Le domaine à plans radiaux est intégré dans Auto et Surfaces. A-084 et A-085 portent les essais du build. Le laboratoire retrouve sur Flansch onze secteurs à partition source vérifiée et réimporte la ceinture inférieure réelle constituée de sept tores et cinq bandes cylindriques. Les deux contours libres de cette ceinture doivent encore être raccordés au plan de base et aux parois du corps. Le helper refuse toujours le document à 231 608 faces. Le jalon global reste en cours.
