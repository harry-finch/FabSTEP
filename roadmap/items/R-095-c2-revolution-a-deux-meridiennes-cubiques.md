---
id: R-095
titre: "C2 — révolution à deux méridiennes cubiques tangentes"
resume: "Reconstruire une paroi de révolution dont la méridienne comporte deux segments cubiques à tangente commune."
statut: a-tester
jalon: post-v1
cree: 2026-11-18
maj: 2026-11-18
livree:
---
## But

Reconstruire une paroi de révolution dont la méridienne comporte deux segments cubiques à tangente commune.

## Inclus

Deux disques d’extrémité, une couture sur un anneau source identifié sans ambiguïté, un seul mur de révolution, provenance de chaque triangle et borne continue.

## Hors périmètre

Les pôles, replis axiaux et cloisons partielles n’entrent pas dans cette preuve.

## Dépendances

R-033, V-1223.

## Critères d'acceptation

Deux STL de densités et de poses différentes réimportent un solide à trois faces. Rejeter une cassure de tangente et préserver les modèles mono-travée.

## Vérifications de l'utilisateur

- Vérifier la révolution à deux segments dans Fusion.
