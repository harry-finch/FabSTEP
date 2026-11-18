---
id: R-091
titre: "F2 — perçages orthogonaux de rayons plus proches"
resume: "Deux trous orthogonaux de rayons plus proches reconstruits. Sept STEP contrôlés dans FreeCAD, y compris quatre maillages indépendants. Essai FabSTEP et Fusion ouvert."
statut: a-tester
jalon: post-v1
cree: 2026-11-18
maj: 2026-11-18
livree:
---
## But

Reconstruire deux perçages centrés et perpendiculaires dans un bloc lorsque leurs rayons sont plus proches que ceux admis dans R-083.

## Inclus

Étendre le domaine au rapport petit rayon / grand rayon strictement inférieur à deux tiers avec marge pour les incertitudes mesurées. Conserver deux courbes communes, la sélection des trois parois cylindriques et le contrôle de la matière après réimport.

## Hors périmètre

Axes décentrés ou obliques, rayons égaux, tangences et intersections multiples restent dans R-036.

## Dépendances

Chantier F demandé par l’utilisateur, contrat existant A-045 et certificat des perçages sécants R-063/R-083. Les mêmes données géométriques décrivent les deux trous.

## Critères d'acceptation

Obtenir un solide fermé à six plans et trois cylindres sur le témoin précédemment facettisé et sur des rapports plus proches du seuil. Vérifier les limites par le certificat natif, le volume par une référence indépendante, l’attribution de tous les triangles et les placements en document multisolide. Vérifier les refus à la frontière, au-delà et sur une source déformée.

## Vérifications de l'utilisateur

- Inspecter les deux perçages dans FabSTEP puis vérifier leurs rayons et leur communication dans Fusion.

## Résultats

Le témoin historique passe de 306 plans à six plans et trois cylindres. Trois rapports et quatre maillages FreeCAD indépendants passent les contrôles de réimport. Auto conserve un solide facettisé au seuil exact et au-delà. Le petit STL FreeCAD de 884 triangles, les autres témoins et les commandes de reproduction sont dans `fixtures/cross_drill/` et V-1219.
