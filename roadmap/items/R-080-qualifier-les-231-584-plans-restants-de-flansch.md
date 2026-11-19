---
id: R-080
titre: "Qualifier les 231 584 plans restants de Flansch"
resume: "Après les quatre grandes zones reconstruites, Flansch conserve 231 584 faces planes et 24 surfaces analytiques. Identifier les zones encore facettisées et leurs familles possibles avant toute nouvelle extension ou proposition de plafond ; la validation native dense reste un obstacle séparé."
statut: livree
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-19
livree: 2026-11-19
---

## But

Localiser et quantifier les zones qui empêchent Flansch de passer sous le plafond des 100 000 faces.

## Inclus

Rejeu du helper, attribution par triangle source, connexité des faces planes, mesures de supports courbes et du potentiel de réduction.

## Hors périmètre

Implémentation d’une nouvelle famille, modification des tolérances ou plafonds, export STEP.

## Dépendances

R-076 et build de développement V-1206.

## Critères d'acceptation

Comptage concordant avec le helper et les propriétaires core, résultats reproductibles, distinction entre hypothèses de surface et secteurs reconstructibles. V-1207 documente les onze supports des deux bandes, le cas défavorable de la troisième et la suite R-085.

## Vérifications de l'utilisateur

Examiner les mesures V-1207 et la priorité proposée à partir des deux bandes de rayon 0,5 mm.
