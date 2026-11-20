---
id: R-126
titre: "Reconstruire une révolution à méridienne mixte"
resume: "G4 · Reconstituer sur un axe commun des portées, des gorges circulaires et des portions de profil cubiques."
statut: livree
jalon: post-v1
cree: 2026-11-20
maj: 2026-11-20
livree: 2026-11-20
---
## But

Des arbres et poulies combinent sur un seul profil des épaulements, un chanfrein et une gorge. Leur reconstruction doit préserver les ruptures de pente qui rendent ces détails usinables.

## Inclus

- Suivre la méridienne paramétrée en rayon et cote axiale, y compris un retour axial. Ajuster des segments, arcs et cubiques bornées.
- Vérifier la coïncidence aux jonctions. Exiger la tangence seulement là où les données la démontrent. Conserver les supports analytiques les plus simples prouvés.

## Hors périmètre

Révolutions à axes multiples, surfaces hélicoïdales et lissage automatique des arêtes vives.

## Dépendances

Corpus R-122 et domaines de révolution existants, dont R-084 et R-111. Peut avancer indépendamment de R-125.

## Critères d'acceptation

- Sur un arbre étagé et une poulie à gorge avec un tronçon courbe, vérifier après réimport les types de faces successifs, les diamètres, la position des ruptures et le retour axial sur les deux densités.
- Rejeter la proposition sur une section excentrée ou sur une quasi-gorge hors tolérance. Vérifier la fermeture et les écarts dans les deux sens depuis les triangles source.

## Vérifications de l'utilisateur

Inspecter dans Fusion le fond de gorge, son retour axial et l’arête vive d’un épaulement.
