---
id: R-093
titre: "Auto — reconnaître le trou traversant d’une plaque inclinée"
resume: "Sur la plaque inclinée de 268 triangles, le mode Automatique reconstruit six faces planes et un cylindre de Ø6 mm. Sélectionner la paroi retrouve ses 126 triangles source."
statut: a-tester
jalon: post-v1
cree: 2026-11-18
maj: 2026-11-18
livree:
---
## But

En mode Automatique, reconstruire la paroi cylindrique d’une plaque inclinée dont le trou traversant est mesurable dans le STL. Avant R-093, le témoin de V-1220 donnait 252 faces planes en Auto et sept faces en mode Cylindres. Les quatre maillages de V-1221 donnent désormais six faces planes et un cylindre en Auto.

## Inclus

- Aux réglages initiaux, retrouver le trou de Ø6 mm dans le bloc incliné de 24 × 20 × 6 mm.
- Permettre de sélectionner la paroi du cylindre dans le STEP et de retrouver ses facettes source.
- Contrôler la pose, les entrées du trou, la matière, le volume et la fidélité sur plusieurs maillages.

## Hors périmètre

Les preuves propres aux perçages qui se croisent relèvent de R-036. La généralisation aux profils libres, trous obliques par rapport à la plaque et parois incomplètes reste en dehors de cet essai.

## Dépendances

R-092 et V-1220 fournissent la pose et la pièce de reproduction. Le modèle de résultat Auto et sa correspondance source sont ceux de R-057.

## Critères d'acceptation

Réimport indépendant d’un solide fermé à six plans et un cylindre. Conservation des 268 triangles du témoin et attribution exacte des triangles de la paroi. Maintien des contrôles de contours, des tolérances, du budget de fidélité et du refus des géométries non prouvées. Contrôle des régressions dans les autres modes.

## Vérifications de l'utilisateur

- Reconstruire la plaque inclinée en Auto, sélectionner la paroi du trou puis vérifier son diamètre dans Fusion.
