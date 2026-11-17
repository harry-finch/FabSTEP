---
id: R-061
titre: "C2a — deux raccords toriques sur une portée"
resume: "Reconstruire un profil exact arc–ligne–arc, avec deux congés toriques et une portée cylindrique commune."
statut: a-tester
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-17
livree:
---

## But

Reconstruire un premier profil de révolution à deux arcs circulaires reliés par une ligne, sans remplacer les arrondis par des facettes. Première tranche de R-033, dans le chantier demandé le 17 novembre.

## Inclus

- Deux quarts de tore extérieurs sur un tour complet, entre deux plans source, séparés par une portée cylindrique résolue.
- Rayons des deux congés éventuellement différents ; une seule géométrie commune de la portée et des raccords.
- Témoins source intérieurs, limites et tangences démontrées ; bornes continues existantes et réimport du solide entier.
- Modes Surfaces et Auto, provenance source, corpus indépendant, refus explicites des cas ambigus ; chemin historique à un tore conservé.

## Hors périmètre

Méridienne libre, arc arbitraire, plusieurs portées/congés, secteurs angulaires, pôles, surplombs et raccords coniques restent dans R-033. Aucun nouveau type de surface ni dépendance.

## Dépendances

Preuves de tores, cylindres et modèle RoundedModel existants. Écriture de rounded.rs après libération par R-060.

## Critères d'acceptation

- Deux plans, un cylindre et deux tores dans le résultat nominal ; paramètres, volume, bornes et fidélité conformes au corpus indépendant.
- Canonicalisation commune des deux raccords ; aucun écrasement successif incompatible du rayon cylindrique.
- Refus des portées absentes/non résolues, courbes déformées, tangences non démontrées et budgets dépassés.
- Régressions un tore et autres familles conservées ; tests et revue indépendante réussis.

## Vérifications de l'utilisateur

- Examiner les deux raccords et leur portée dans le build de développement.
- Réimporter le STEP dans Fusion et vérifier les deux surfaces toriques et les dimensions.

## Avancement — 17 novembre 2026

Deux quarts de tore extérieurs avec rayon de portée commun et résidus source contrôlés ; variantes et refus qualifiés. Revue indépendante acceptée. Les raccords intérieurs restent hors domaine. Validation globale V-1186 : 459 tests workspace, 32 bureau, 208 UI, 12 Python et 4 SDK ; formatage, analyse statique et build macOS réussis. Version à essayer : `artifacts/FabSTEP.app`. Les actions utilisateur associées séparent examen dans FabSTEP et réimport Fusion.
