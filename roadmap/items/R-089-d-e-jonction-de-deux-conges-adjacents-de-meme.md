---
id: R-089
titre: "D/E — jonction de deux congés adjacents de même rayon"
resume: "Deux congés perpendiculaires se rejoignent le long d’un arc d’ellipse. Le témoin FreeCAD possède deux cylindres, encore facettisés dans Auto."
statut: en-cours
jalon: post-v1
cree: 2026-11-18
maj: 2026-11-18
livree:
---
## But

Obtenir deux faces cylindriques sélectionnables quand deux arrondis de même rayon se rejoignent dans un coin du fond d’une poche rectangulaire.

## Inclus

Une poche borgne, un fond plan, deux quarts de cylindre concaves de même rayon dont les axes sont perpendiculaires et une seule arête elliptique commune. Une butée plane droite termine chaque congé à son autre extrémité. Qualifier d’abord la jonction sur une pièce de référence.

## Hors périmètre

Rayons différents, trois arêtes arrondies au même sommet, raccords sphériques ou à rayon variable et autres angles entre les axes.

## Dépendances

R-086 pour les congés à butées droites, R-087 pour les bords elliptiques et R-088 pour les congés opposés. L’intégration dans le contrat core/CAO requiert de vérifier l’incidence de la même ellipse sur deux cylindres et d’obtenir l’accord prévu par INT-AUDIT-01.

## Critères d'acceptation

Après conversion, réimporter un solide à onze plans et deux cylindres. Prouver chaque bande source, l’arête elliptique unique et son tracé sur chacune des deux faces, les deux cercles d’arrêt, la matière et le volume. Rejeter une jonction déformée au-delà de la tolérance. Préserver la provenance et vérifier plusieurs triangulations et orientations ainsi qu’une pièce CAO indépendante.

## Vérifications de l'utilisateur

- Sélectionner séparément les deux congés adjacents dans FabSTEP et vérifier leur raccord commun dans Fusion.

## Qualification initiale

Les trois STL publics sont reproductibles. Une seconde pièce construite dans FreeCAD donne une seule ellipse partagée, de demi-axes 3,535534 et 2,5 mm. Auto réimporte encore 71 ou 159 plans selon le maillage. Le domaine proposé et les critères de preuve figurent dans [ADR-0042](../../docs/adr/0042-adjacent-pocket-fillets.md). Voir V-1215.
