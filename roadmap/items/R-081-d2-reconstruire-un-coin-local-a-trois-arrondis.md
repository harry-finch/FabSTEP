---
id: R-081
titre: "D2 — reconstruire un coin local à trois arrondis"
resume: "Intégrer l’octant sphérique et ses trois raccords cylindriques à partir des frontières STL, avec preuve continue et STEP validé."
statut: livree
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-19
livree: 2026-11-19
---
## But

Reconstruire un coin local à trois arrondis de même rayon, au lieu d’exiger les douze arêtes d’un boîtier entièrement arrondi. Première intégration source du domaine D approuvé dans ADR-0036 et A-068.

## Inclus

- Bloc orthogonal à six supports plans réels, un trièdre arrondi constitué de trois quarts de cylindre et d’un octant sphérique de rayon commun, extrémités présentes sur les plans opposés.
- Hypothèse géométrique unique issue des frontières source ; dimensions, rayon, tangences et côté matière démontrés.
- Trois arcs partagés entre octant et cylindres, sans ajout d’arrondi sur les autres arêtes.
- Couverture source complète et orientée, résidus vectoriels et bornes continues pour sphère, cylindres et raccords aux plans.
- Auto et Surfaces, réimport STEP, volume/dimensions, audit et provenance exacte ; modèle et ABI additifs, anciens contrats conservés.

## Hors périmètre

Plusieurs trièdres locaux combinés, coins non orthogonaux, rayons différents, cavités concaves, perçages au contact du raccord et propagation implicite. Le générateur de laboratoire seul ne constitue pas une reconnaissance. Les autres parties de D restent suivies dans R-034.

## Dépendances

R-079, ADR-0036 et A-068 approuvés. Base C1/C2/E2 validée avant intégration ; écritures core/CAO sérialisées avec G.

## Critères d'acceptation

- Corpus indépendant positif, transformé et retessellisé, avec cas négatifs de raccords absents, rayon incohérent, couverture manquante ou ambiguë.
- Six plans, trois cylindres partiels et une face sphérique locale, un solide réimporté ; volume analytique et dimensions cohérents.
- Chaque triangle source attribué une fois ; trois vrais arcs d’incidence deux et sens opposés ; aucune face ou fermeture inventée.
- Dépassements de ressources et déviation refusés, aucun ancien résultat exportable après mutation ; guidage incompatible explicitement refusé sans réutiliser un ancien export.
- Revue indépendante, tests core/natifs/provenance et régressions réussis.

## Vérifications de l'utilisateur

- Examiner le coin arrondi dans FabSTEP, ses trois raccords et sa sélection de faces.
- Réimporter le STEP de référence dans Fusion et vérifier les dix faces, dimensions et volume.

## Plan d’intégration

1. Ajouter l’octant au modèle arrondi avec une liste optionnelle par défaut vide, un centre/rayon, trois directions orthonormales directes et trois usages orientés des arcs source. Conserver les symboles CAO existants ; utiliser une entrée additive lorsque le modèle contient un octant. Le premier domaine reste convexe.
2. Retrouver le trièdre depuis six supports plans et leurs vraies frontières. Classer chaque triangle une fois ; démontrer la couverture de l’octant par un domaine triangulaire projectif orienté, et celle des trois cylindres par leurs domaines UV. Compter les écarts sur les frontières courbes des plans.
3. Construire les faces avec arêtes natives réellement partagées ; intégrer le flux de volume, la provenance et les chemins de guidage. Refuser explicitement toute correction qui ne revalide pas les voisins couplés.
4. Tester les sources indépendantes (nominale, miroir, permutation, rotation et autre tessellation), les défauts et les plafonds, puis relire le diff et exécuter les contrôles globaux avant de proposer le build.

Écritures disjointes prévues : core/reconnaissance, adaptateur CAO, corpus/tests d’intégration, orchestration applicative/documentation. Les exécutions Cargo restent sérialisées.
