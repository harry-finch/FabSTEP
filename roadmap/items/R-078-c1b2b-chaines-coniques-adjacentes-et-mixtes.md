---
id: R-078
titre: "C1b2b — chaînes coniques adjacentes et mixtes"
resume: "Reconstruire deux à huit cônes adjacents ou mêlés à des portées, avec une décomposition unique prouvée par les anneaux source."
statut: livree
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-19
livree: 2026-11-19
---
## But

Poursuivre C1b2 au-delà de la paire de cônes de R-060 : reconnaître une chaîne bornée de cônes adjacents et ses éventuelles portées cylindriques, lorsque chaque changement de pente est démontré. La poursuite globale de C1b2 est autorisée par l’utilisateur.

## Inclus

- Deux à huit cônes coaxiaux de rayons positifs, avec ruptures sur des anneaux source complets et deux témoins intérieurs résolus par portion.
- Priorité aux lois affines et aux paires déjà prouvées ; recherche bornée d’une décomposition unique pour les chaînes plus longues.
- Combinaisons de portions coniques adjacentes et de portées cylindriques résolues, sans changer les données ou la CAO.
- Modes Primitives, Surfaces et Auto, corpus indépendant, validation native et provenance exacte.

## Hors périmètre

Bandes à deux anneaux sans preuve contextuelle, méridiennes lisses interprétées comme une suite de cônes, pôles, cônes partiels, nouvelles tolérances et contrat public. Les chaînes avec tores sont traitées dans C2.

## Dépendances

R-032 et R-060 ; intégration sérialisée avec R-076 dans le reconnaisseur arrondi.

## Critères d'acceptation

- Ruptures uniques et pentes séparées au-delà des incertitudes ; tout autre découpage admissible refuse la nouvelle chaîne.
- Témoins insuffisants, transitions ambiguës, courbure, frontières traversées et budgets dépassés restent refusés.
- Volume, dimensions, solides et nombre de cônes attendus après réimport ; partition source exhaustive.
- Corpus à trois et huit cônes, signes variés, portées et placements ; régressions historiques et revue réussies.

## Vérifications de l'utilisateur

- Examiner les chaînes coniques de référence et leurs anneaux dans FabSTEP.
- Vérifier leurs STEP dans Fusion avec volumes et dimensions attendus.
