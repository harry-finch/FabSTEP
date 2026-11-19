---
id: R-089
titre: "D/E — jonction de deux congés adjacents de même rayon"
resume: "Dans un coin de poche à 90°, Auto reconstruit les deux congés de même rayon et conserve leur arête elliptique commune."
statut: livree
jalon: post-v1
cree: 2026-11-18
maj: 2026-11-19
livree: 2026-11-19
---
## But

Obtenir deux faces cylindriques sélectionnables quand deux arrondis de même rayon se rejoignent dans un coin du fond d’une poche rectangulaire.

## Inclus

Une poche borgne, un fond plan, deux quarts de cylindre concaves de même rayon dont les axes sont perpendiculaires et une seule arête elliptique commune. Une butée plane droite termine chaque congé à son autre extrémité. Qualifier d’abord la jonction sur une pièce de référence.

## Hors périmètre

Rayons différents, trois arêtes arrondies au même sommet, raccords sphériques ou à rayon variable et autres angles entre les axes.

## Dépendances

R-086 pour les congés à butées droites, R-087 pour les bords elliptiques et R-088 pour les congés opposés. L’accord A-089 autorise l’extension du contrat core/CAO à la même ellipse sur deux cylindres.

## Critères d'acceptation

Après conversion, réimporter un solide à onze plans et deux cylindres. Prouver chaque bande source, l’arête elliptique unique et son tracé sur chacune des deux faces, les deux cercles d’arrêt, la matière et le volume. Rejeter une jonction déformée au-delà de la tolérance. Préserver la provenance et vérifier plusieurs triangulations et orientations ainsi qu’une pièce CAO indépendante.

## Vérifications de l'utilisateur

- Sélectionner séparément les deux congés adjacents dans FabSTEP et vérifier leur raccord commun dans Fusion.

## Qualification initiale

Les trois STL de qualification initiale sont reproductibles. Une seconde pièce construite dans FreeCAD donne une seule ellipse partagée, de demi-axes 3,535534 et 2,5 mm. Avant intégration, Auto réimportait 71 ou 159 plans selon le maillage. Le domaine approuvé et les critères de preuve figurent dans [ADR-0042](../../docs/adr/0042-adjacent-pocket-fillets.md). Voir V-1215.

## Résultat de développement

Auto et Surfaces reconstruisent onze plans et deux cylindres sur les témoins publics, y compris la poche courte, et sur deux maillages indépendants. Les contrôles couvrent les voisins, la provenance, le DXF des butées, la fidélité, le partage de l’ellipse après réimport et la séparation des corps. Le coin déformé conserve ses facettes. Voir [V-1216](../../docs/VALIDATION.md#v-1216--r-089-intégré-step-de-la-poche-à-congés-adjacents).
