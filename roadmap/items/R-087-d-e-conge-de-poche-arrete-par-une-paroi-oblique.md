---
id: R-087
titre: "D/E — congé de poche arrêté par une paroi oblique"
resume: "Reconstruire le congé cylindrique entre une butée droite et une butée inclinée et vérifier son arc elliptique."
statut: en-cours
jalon: post-v1
cree: 2026-11-18
maj: 2026-11-18
livree:
---
## But

Reconstruire le congé cylindrique d’une poche lorsque l’une de ses parois d’arrêt est inclinée. Dans le STEP, le congé doit former une seule face et la paroi inclinée doit porter son contour elliptique exact.

## Inclus

Un congé intérieur de rayon constant tangent au fond et à une paroi, entre une butée droite et une butée plane oblique. Contrôler des inclinaisons de signes opposés, une autre triangulation et une poche dessinée indépendamment en CAO.

## Hors périmètre

Deux butées obliques, intersections de plusieurs congés, rayon variable et conversion intégrale de Flansch.

## Dépendances

R-086 pour les congés de fond et R-082 pour la représentation des arcs elliptiques. Le cylindre partiel actuel se termine par deux arcs de cercle. La prise en charge d’une terminaison elliptique dans le contrat core/CAO est soumise à arbitrage.

## Critères d'acceptation

Vérifier un solide fermé après réimport, son volume, ses douze faces dont un seul cylindre, sa concavité et ses quatre voisins. Retrouver sur la butée inclinée un arc d’ellipse aux dimensions attendues et sur l’autre butée un arc de cercle. Vérifier les limites de fidélité sur chaque triangle, l’attribution exhaustive et la conservation du vide. Refuser la reconstruction analytique sur une paroi déformée au-delà des tolérances.

## Vérifications de l'utilisateur

- Examiner la poche dans FabSTEP, puis contrôler les deux contours d’arrêt dans le STEP réimporté dans Fusion.

## État de la qualification

Auto produit 46 plans sur le témoin nominal. Le STEP de référence construit séparément dans FreeCAD comporte un cylindre joint à ses deux butées, dont les profils DXF passent dans le noyau de FabSTEP. Les mesures et le corpus indépendant sont dans [V-1212](../../docs/VALIDATION.md#v-1212--r-087-cylindre-de-poche-avec-une-butée-oblique). A-087 porte sur l’extension du contrat nécessaire à la reconstruction depuis le STL, décrite dans [ADR-0041](../../docs/adr/0041-oblique-cylinder-pocket.md).
