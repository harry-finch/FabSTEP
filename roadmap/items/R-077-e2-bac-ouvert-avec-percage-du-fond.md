---
id: R-077
titre: "E2 — bac ouvert avec perçage du fond"
resume: "Qualifier un bac à coins arrondis avec un trou traversant le fond, sans remplir la cavité ni perdre les parois."
statut: a-tester
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-17
livree:
---
## But

Étendre E au bac ouvert percé, en préservant le fond restant, les parois arrondies et la communication entre le trou et la cavité. Cette tranche réutilise les primitives existantes avant de concevoir les congés de plancher/rebord.

## Inclus

- Bac ouvert de référence R-062 avec un perçage cylindrique traversant le fond, éloigné des arrondis.
- Variantes déplacées/tournées et cas de frontière absente ou de trou déformé.
- Vérification de la combinaison des cylindres complets et partiels, du vide, du volume, des dimensions et de la provenance native exacte.
- Corrections locales seulement si un défaut est reproduit ; aucune nouvelle représentation géométrique.

## Hors périmètre

Perçages sécants, trous touchant les arrondis, congés du plancher/rebord, cavités entièrement enfermées et contacts entre corps. Aucun fond inventé ni export partiel.

## Dépendances

R-062 et modèles mixtes existants. Accord utilisateur pour poursuivre l’ensemble des lots, dont E.

## Critères d'acceptation

- Source topologiquement fermée ; résultat solide réimporté avec onze plans et neuf surfaces cylindriques pour le nominal.
- Volume analytique du bac diminué du cylindre retiré, dimensions inchangées et sondes matière/vide conformes.
- Attribution exhaustive et disjointe des triangles source ; paramètres et bornes du perçage contrôlés.
- Source ouverte refusée ; aucune attribution cylindrique fausse sur le trou déformé.
- Générateur reproductible, tests natifs et régressions adaptés réussis.

## Vérifications de l'utilisateur

- Examiner le bac percé dans FabSTEP et vérifier que le fond et le perçage sont conservés.
- Réimporter le STEP du bac percé dans Fusion et vérifier volume et surfaces.
