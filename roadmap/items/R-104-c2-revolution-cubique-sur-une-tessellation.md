---
id: R-104
titre: "C2 — Révolution cubique sur une tessellation irrégulière"
resume: "Reconstruire une méridienne cubique lorsque le maillage FreeCAD ne présente pas d’anneaux intérieurs complets."
statut: a-tester
jalon: post-v1
cree: 2026-11-19
maj: 2026-11-19
livree:
---
## But

Reconstruire une méridienne cubique lorsque le maillage FreeCAD ne présente pas d’anneaux intérieurs complets.

## Inclus

Deux disques source réels, profil positif mono-travée et contrôle de la projection radiale de chaque triangle.

## Hors périmètre

Axes de révolution ambigus, retour axial ou dépassement de la déviation demandée.

## Dépendances

R-033, V-1234.

## Critères d'acceptation

- Deux plans et une révolution après réimport dans les trois cas admissibles.
- Le maillage dont l’écart atteint 0,08637 mm reste hors domaine avec le budget de 0,05 mm.

## Vérifications de l'utilisateur

- Vérifier les révolutions sans anneaux intérieurs dans Fusion.
