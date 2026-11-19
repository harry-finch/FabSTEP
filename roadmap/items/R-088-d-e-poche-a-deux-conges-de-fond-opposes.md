---
id: R-088
titre: "D/E — poche à deux congés de fond opposés"
resume: "Reconstruire et mesurer séparément les deux arrondis parallèles d’une poche borgne, y compris lorsque leurs rayons diffèrent."
statut: livree
jalon: post-v1
cree: 2026-11-18
maj: 2026-11-19
livree: 2026-11-19
---
## But

Retrouver séparément les deux arrondis opposés au fond d’une poche borgne. Dans le STEP, chaque arrondi doit pouvoir être sélectionné et mesuré avec son propre rayon.

## Inclus

Deux congés cylindriques concaves parallèles, séparés par un fond plat et terminés sur deux parois planes perpendiculaires aux axes. Examiner plusieurs triangulations et une seconde pièce dessinée dans FreeCAD.

## Hors périmètre

Congés qui se rejoignent, butées obliques et rayons variables le long d’un congé.

## Dépendances

Les chantiers D et E autorisent les combinaisons de congés prouvés. R-086 couvre un congé arrêté. Les deux surfaces du présent cas utilisent le contrat existant des cylindres partiels.

## Critères d'acceptation

Réimporter un seul solide comportant onze plans et deux quarts de cylindre concaves de rayons distincts. Vérifier le volume, les dimensions, les quatre voisins de chaque congé, les deux arcs de chaque butée, la matière et le vide. Contrôler l’attribution exhaustive des triangles en Auto et la fidélité. Une bande localement déformée doit conserver la géométrie justifiée par ses triangles.

## Vérifications de l'utilisateur

- Sélectionner séparément les deux congés dans FabSTEP, puis mesurer leurs rayons dans le STEP réouvert dans Fusion.

## Résultat

Le moteur du commit `789fdf5` couvrait déjà ce cas. Les trois variantes positives sont protégées par les tests de réimport, de provenance et de DXF. Un sommet déplacé dans le congé droit conserve le cylindre gauche. Les mesures et la reproduction indépendante figurent dans [V-1214](../../docs/VALIDATION.md#v-1214--deux-congés-de-fond-opposés-r-088).
