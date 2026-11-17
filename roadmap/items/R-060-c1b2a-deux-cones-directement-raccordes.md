---
id: R-060
titre: "C1b2a — deux cônes directement raccordés"
resume: "Reconstruire deux cônes adjacents lorsque leur rupture unique et leurs parois sont prouvées par les anneaux source."
statut: a-tester
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-17
livree:
---

## But

Reconstruire la rupture entre deux cônes directement adjacents, aujourd’hui refusée faute de portée intermédiaire. Première tranche de R-059, dans la poursuite géométrique explicitement demandée le 17 novembre.

## Inclus

- Deux troncs de cône complets, de rayons positifs, coaxiaux et de pentes signées distinctes, entre limites circulaires planes source.
- Une seule rupture admissible sur un anneau source complet ; témoins intérieurs dans chaque portion et preuve complète de chaque paroi.
- Priorité à la loi affine unique déjà reconnue ; refus des splits multiples ou numériquement ambigus, sans assouplissement des tolérances.
- Modes Primitives, Surfaces et Auto, provenance exhaustive, réimport STEP solide, volume, dimensions et fidélité ; corpus indépendant reproductible.

## Hors périmètre

Trois cônes ou davantage sans portée, pôles, portées terminales/mélangées et profils libres restent dans R-059/R-033. Aucun nouveau DTO, ABI, dépendance ou comportement d’export.

## Dépendances

R-032 et preuves natives existantes. Les validations utilisateur de R-032 ne sont pas remplacées par cette extension.

## Critères d'acceptation

- Deux surfaces coniques et un anneau partagé prouvés, avec volume nominal et attribution source exacte après réimport.
- Refus des profils courbes, des témoins insuffisants, des ruptures ambiguës, des triangles traversant la rupture et des dépassements de budgets.
- Compatibilité des chaînes historiques ; le corpus no_land suffisamment échantillonné devient explicitement positif, sans modifier ses octets STL.
- Tests ciblés et globaux, analyse statique et build bureau réussis ; revue indépendante du contrat et du diff.

## Vérifications de l'utilisateur

- Essayer les modèles à deux cônes raccordés dans le build de développement et inspecter leurs surfaces.
- Réimporter un STEP du corpus dans Fusion et vérifier solide, dimensions et anneau de raccord.

## Avancement — 17 novembre 2026

Deux cônes adjacents prouvés dans les deux reconnaisseurs existants ; corpus indépendant et provenance native exacte. Revue indépendante acceptée. Aucun nouveau DTO ni changement de tolérance. Validation globale V-1186 : 459 tests workspace, 32 bureau, 208 UI, 12 Python et 4 SDK ; formatage, analyse statique et build macOS réussis. Version à essayer : `artifacts/FabSTEP.app`. Les actions utilisateur associées séparent examen dans FabSTEP et réimport Fusion.
