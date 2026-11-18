---
id: R-096
titre: "F — perçages traversants orthogonaux à axes décentrés"
resume: "Reconstruire les deux intersections partagées lorsque le petit perçage est décentré par rapport au grand."
statut: a-tester
jalon: post-v1
cree: 2026-11-18
maj: 2026-11-18
livree:
---
## But

Reconstruire les deux intersections partagées lorsque le petit perçage est décentré par rapport au grand.

## Inclus

Quatre bouches source, deux branches transverses, petit axe déplacé dans le plan de sa section, volume de la matière et provenance.

## Hors périmètre

Tangence, diamètres presque égaux, axes non orthogonaux ou intersection multiple. Les bouches doivent être identifiables dans le STL.

## Dépendances

R-036, V-1224.

## Critères d'acceptation

STEP fermé à six plans et trois cylindres. Contrôler les deux arêtes communes par encadrement et les ouvertures, le volume et les points de matière dans FreeCAD.

## Vérifications de l'utilisateur

- Vérifier les deux perçages décentrés dans Fusion.
