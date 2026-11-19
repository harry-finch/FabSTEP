---
id: R-086
titre: "D/E — poche borgne à congé de fond arrêté"
resume: "Auto reconstruit un congé intérieur à deux butées, même quand ses extrémités comportent des nombres de facettes différents."
statut: livree
jalon: post-v1
cree: 2026-11-18
maj: 2026-11-19
livree: 2026-11-19
---
## But

Reconstruire une poche borgne dont une seule jonction entre le fond et une paroi est arrondie. Le rayon et les deux arrêts du congé doivent pouvoir être retrouvés dans le STEP.

## Inclus

Une poche rectangulaire à fond présent, un congé intérieur de rayon constant et deux parois d’arrêt perpendiculaires à son axe. Vérifier une pièce tournée, un autre maillage et une poche de dimensions différentes avec le congé du côté opposé.

## Hors périmètre

Arrêts obliques, rayon variable, jonction entre plusieurs congés et couture complète de Flansch.

## Dépendances

Chantiers D (R-034) et E (R-035), représentation existante des cylindres partiels et précédent du bac ouvert R-077.

## Critères d'acceptation

Le parcours Auto retrouve un solide fermé, onze faces planes et un quart de cylindre concave. Vérifier la matière, le vide, le volume, l’attribution des facettes, la fidélité et les quatre faces voisines après réimport STEP. Une ouverture dans le fond doit entraîner un refus. Une déformation au-delà des tolérances doit empêcher la reconnaissance du congé nominal.

## Vérifications de l'utilisateur

- Inspecter le congé au fond de la poche dans FabSTEP et ses arrêts dans le STEP réimporté dans Fusion.

## Résultat de développement

Auto réimporte un solide à onze plans et un cylindre sur les cinq variantes analytiques du corpus. La poche dont les extrémités ont 16 et 23 intervalles donnait auparavant 50 faces planes. Les preuves couvrent également un congé extérieur et deux STL produits par FreeCAD sur une autre poche. Une rotation dont les coordonnées binaires arrondies dépassent la précision requise aux jonctions conserve un solide facettisé. Voir V-1211 dans [VALIDATION](../../docs/VALIDATION.md).
