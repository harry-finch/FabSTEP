---
id: R-106
titre: "Révolution à deux cubiques sur un maillage irrégulier"
resume: "Reconstruire une méridienne à deux segments tangents dont le raccord se trouve entre deux hauteurs du STL."
statut: a-tester
jalon: post-v1
cree: 2026-11-19
maj: 2026-11-19
livree:
---
## But

Reconstruire une méridienne à deux segments tangents dont le raccord se trouve entre deux hauteurs du STL.

## Inclus

Deux cubiques tangentes, deux orientations et deux densités.

## Hors périmètre

Profils avec retour axial ou cassure.

## Dépendances

R-033, V-1236.

## Critères d'acceptation

Quatre STEP à trois faces et refus du témoin à tangente discontinue.

## Vérifications de l'utilisateur

- Vérifier la révolution sur les STL two_span et two_span_turned.
