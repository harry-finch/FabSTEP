---
id: R-112
titre: "Intégrer les recommandations NEON vérifiées"
resume: "Suivi des seize constats et des reports dans docs/INT_AUDIT_01.md, section NEON. Contrôle atlas restant lié à R-085."
statut: en-cours
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

Les travaux différés sont le contrôle d’interférence équivalent de R-085, la mutualisation de la topologie et de la recherche de paramètres, les projections locales, le BVH de sélection, le démarrage WebView et la qualification Windows. Le registre précise les preuves attendues avant reprise.

## Dépendances

Le rapport du helper et le manifeste du paquet demandent un arbitrage de compatibilité. Les autres lots sont indépendants.

## Critères d'acceptation

Tests par lot et vérification des décisions géométriques, de la fraîcheur des exports, des budgets existants et des mesures avant/après. Consigner chaque limite dans le registre.

## Vérifications de l'utilisateur

Tester dans le bureau la peinture au clavier et un export après modification de tolérance. Les essais Fusion déjà prévus restent associés aux jalons géométriques.
