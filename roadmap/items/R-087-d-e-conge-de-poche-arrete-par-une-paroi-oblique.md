---
id: R-087
titre: "D/E — congé de poche arrêté par une paroi oblique"
resume: "Auto reconstruit le congé cylindrique entre une butée droite et une paroi oblique. Les profils de face conservent le cercle et l’ellipse."
statut: livree
jalon: post-v1
cree: 2026-11-18
maj: 2026-11-19
livree: 2026-11-19
---
## But

Reconstruire le congé cylindrique d’une poche lorsque l’une de ses parois d’arrêt est inclinée. Dans le STEP, le congé doit former une seule face et la paroi inclinée doit porter son contour elliptique exact.

## Inclus

Un congé intérieur de rayon constant tangent au fond et à une paroi, entre une butée droite et une butée plane oblique. Contrôler des inclinaisons de signes opposés, une autre triangulation et une poche dessinée indépendamment en CAO.

## Hors périmètre

Deux butées obliques, intersections de plusieurs congés, rayon variable et conversion intégrale de Flansch.

## Dépendances

R-086 pour les congés de fond et R-082 pour la représentation des arcs elliptiques. L’extension additive du cylindre partiel à une terminaison elliptique est approuvée dans A-087 (ADR-0041).

## Critères d'acceptation

Vérifier un solide fermé après réimport, son volume, ses douze faces dont un seul cylindre, sa concavité et ses quatre voisins. Retrouver sur la butée inclinée un arc d’ellipse aux dimensions attendues et sur l’autre butée un arc de cercle. Vérifier les limites de fidélité sur chaque triangle, l’attribution exhaustive et la conservation du vide. Refuser la reconstruction analytique sur une paroi déformée au-delà des tolérances.

## Vérifications de l'utilisateur

- Examiner la poche dans FabSTEP, puis contrôler les deux contours d’arrêt dans le STEP réimporté dans Fusion.

## État de la qualification

L’accord A-087 autorise l’extension décrite dans [ADR-0041](../../docs/adr/0041-oblique-cylinder-pocket.md). Auto réimporte un solide à onze plans et un cylindre sur les cinq STL publics et sur deux maillages indépendants issus de FreeCAD. Le témoin nominal donnait 46 faces planes avant ce changement. La butée inclinée s’exporte avec un arc elliptique en DXF. Les preuves de conversion, les refus et les limites sont dans [V-1213](../../docs/VALIDATION.md#v-1213--r-087-intégré-step-de-la-poche-à-butée-oblique).
