---
id: R-062
titre: "D1–E1 — arêtes arrondies et bac ouvert"
resume: "Qualifier les arrondis cylindriques locaux et les bacs ouverts à coins verticaux arrondis, avec fond et vide source conservés."
statut: a-tester
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-17
livree:
---

## But

Étendre la qualification aux pièces avec seulement certaines arêtes arrondies et aux bacs ouverts, sans supposer qu’une boîte entière possède le même congé. Première tranche pratique de R-034 et R-035 dans le chantier demandé le 17 novembre.

## Inclus

- Boîte avec une puis deux arêtes droites disjointes arrondies, extrémités planes réellement présentes.
- Bac ouvert avec quatre coins verticaux extérieurs et quatre intérieurs cylindriques, fond, plancher et rebord présents ; transitions du fond et du rebord vives.
- Corpus analytique indépendant, paramètres de tangence, volumes et vide intérieur connus ; variantes transformées et cas négatifs.
- Vérification des capacités existantes avant toute modification du moteur ; corrections locales uniquement si un défaut est reproduit.

## Hors périmètre

Coins à trois branches et congés du fond/rebord restent R-034/R-035 ; perçages sécants R-036, rayons variables R-037 et cavités enfermées à plusieurs coques sont distincts. Aucun élargissement des contacts entre corps ni fermeture inventée.

## Dépendances

Modèle mixte et cylindres partiels existants. Les preuves de tangence des corpus ne transforment pas toutes les parois cylindriques reconnues en congés certifiés.

## Critères d'acceptation

- Surfaces partielles analytiques, provenance exhaustive et solide fermé après réimport.
- Volume nominal et dimensions conservés ; fond et cavité non comblés, pas d’export partiel.
- Cas avec frontière manquante ou déformation refusés dans le mode explicite ; aucun rayon variable interprété comme un cylindre valide.
- Défauts éventuels corrigés sans modifier le contrat des arcs historiques ; tests et limites documentés.

## Vérifications de l'utilisateur

- Ouvrir le bac et les pièces à arêtes arrondies dans FabSTEP et examiner la cavité et les raccords.
- Réimporter un STEP du corpus dans Fusion et vérifier solide, épaisseur et surfaces cylindriques.

## Avancement — 17 novembre 2026

Le moteur existant couvre les pièces retenues : ajout du corpus et des tests, aucune modification de production. Fond et vide vérifiés sur la source et le STEP réimporté ; les coins triédriques et rayons variables restent ouverts. Validation globale V-1186 : 459 tests workspace, 32 bureau, 208 UI, 12 Python et 4 SDK ; formatage, analyse statique et build macOS réussis. Version à essayer : `artifacts/FabSTEP.app`. Les actions utilisateur associées séparent examen dans FabSTEP et réimport Fusion.
