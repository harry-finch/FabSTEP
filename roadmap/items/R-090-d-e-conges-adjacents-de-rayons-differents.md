---
id: R-090
titre: "D/E — congés adjacents de rayons différents"
resume: "Deux congés adjacents de rayons distincts dans une poche rectangulaire à butées droites. Auto produit deux faces cylindriques sélectionnables et conserve l’arc de prolongement en DXF."
statut: a-tester
jalon: post-v1
cree: 2026-11-18
maj: 2026-11-18
livree:
---
## But

Sélectionner séparément les deux congés de rayons différents qui se rejoignent à angle droit au fond d’une poche borgne.

## Inclus

Deux axes orthogonaux parallèles au fond, un quart de cylindre concave de chaque côté, deux rayons constants distincts, un seul coin commun et deux butées droites. Mesurer la branche commune aux cylindres et l’arc supplémentaire du grand congé sur la paroi verticale du petit.

## Hors périmètre

Rayon variable le long d’une arête, surface de lissage ajoutée au coin, intersection de plus de deux congés et axes obliques.

## Dépendances

R-089 décrit le coin de même rayon. L’accord A-092 autorise le nouveau domaine du contrat core/CAO pour la branche commune et le contour à cinq bords.

## Critères d'acceptation

Établir une référence analytique et une pièce CAO indépendante. Après intégration, réimporter un solide fermé de onze plans et deux cylindres, retrouver leurs rayons, l’arête unique partagée, la continuation circulaire et les propriétaires de chaque bord. Contrôler la borne de fidélité, le volume, les dimensions, le vide, la provenance et plusieurs triangulations. Écarter une jonction déformée sans relâcher les tolérances. Vérifier les deux placements du petit rayon et préserver le cas de même rayon.

## Vérifications de l'utilisateur

- Examiner dans FabSTEP les deux faces cylindriques et vérifier leurs rayons et leurs terminaisons sur le STEP rouvert dans Fusion.

## Qualification initiale

Les maillages publics et une autre pièce créée dans FreeCAD confirment le contour à cinq arêtes du grand cylindre. La courbe spatiale admet une quartique rationnelle à cinq pôles. Avant intégration, la conversion depuis les STL livrait une pièce facettisée. Voir [ADR-0043](../../docs/adr/0043-unequal-adjacent-pocket-fillets.md) et [V-1217](../../docs/VALIDATION.md#v-1217--qualification-du-coin-à-deux-rayons-distincts-r-090).

## Intégration

Le build de développement reconstruit deux cylindres sélectionnables et leur jonction non plane. Le cinquième bord du grand congé apparaît comme arc dans le DXF de la paroi voisine. Les contrôles portent sur les maillages publics, deux triangulations FreeCAD, un STL incliné, un coin déformé et plusieurs corps. Le reconnaisseur traite les poches à onze plans sur les corps de 24 à 4 096 triangles. [Validation V-1218](../../docs/VALIDATION.md#v-1218--conversion-stl-des-deux-congés-adjacents-de-rayons-différents-r-090)
