---
id: R-112
titre: "Intégrer les recommandations NEON vérifiées"
resume: "Suivi des seize constats et des reports dans docs/INT_AUDIT_01.md, section NEON. Contrôle atlas restant lié à R-085."
statut: a-tester
jalon: audit
cree: 2026-11-19
maj: 2026-11-19
livree:
---
## But

Intégrer les corrections vérifiées de l’audit NEON du 19 novembre. Le registre détaillé est la section NEON de `docs/INT_AUDIT_01.md`.

## Inclus

Sept jalons couvrent les outils de livraison, la publication atomique, la tessellation, les chemins géométriques mesurés, l’atelier, le rapport du helper et le paquet macOS.

## Hors périmètre

Les travaux différés sont le contrôle d’interférence équivalent de R-085, la mutualisation de la topologie et de la recherche de paramètres, les variantes du calcul de fidélité, les projections locales, le BVH de sélection, le démarrage WebView et la qualification Windows. Le registre précise les preuves attendues avant reprise.

## Dépendances

Les arbitrages A-119 et A-120 ont été validés. Le rapport v2 et le paquet autonome ont été qualifiés avec les versions précédentes.

## Critères d'acceptation

Tests par lot et vérification des décisions géométriques, de la fraîcheur des exports, des budgets existants et des mesures avant/après. Consigner chaque limite dans le registre.

## Vérifications de l'utilisateur

Vérifier la conservation d’une sélection STEP après Échap pendant un geste de peinture, puis le blocage de l’ancien export après édition de la tolérance.
