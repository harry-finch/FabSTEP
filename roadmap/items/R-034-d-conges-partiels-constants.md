---
id: R-034
titre: "D — congés partiels constants"
resume: "Congés de poche à un ou deux arrêts obliques. Deux congés adjacents à rayons égaux se rejoignent également dans des coins aigus ou obtus."
statut: en-cours
jalon: post-v1
cree: 2026-11-13
maj: 2026-11-19
livree:
---

## But

Reconstruire des congés constants locaux sans propager artificiellement un arrondi à toute la pièce.

## Inclus

Arêtes et coins locaux dont les supports, rayons, tangences, arrêts et domaines sont démontrés ; généralisation progressive des combinaisons.

## Hors périmètre

Propagation implicite, raccords arbitraires, absence de supports ou de frontières prouvées.

## Dépendances

R-062, R-079, R-081 ; contrat local ADR-0036 approuvé par A-068.

## Critères d'acceptation

Couverture source exhaustive, borne continue, limites partagées, volume et solide STEP réimporté vérifiés pour chaque domaine intégré.

## Vérifications de l'utilisateur

Les essais de chaque tranche sont portés par ses sous-jalons et leurs actions FabBoard ; ne pas les dupliquer au niveau de ce chantier global.

## État consolidé

Les essais de doubles arrêts obliques sont dans R-097. La jonction elliptique à 65° et 115° est contrôlée dans R-101. Les réseaux comportant plusieurs jonctions partagées restent à qualifier.

Les essais dans Fusion sont suivis dans les actions des sous-jalons.

## Historique et décisions

| D — congés partiels constants | Une ou deux arêtes droites arrondies, puis coin à trois branches | Supports, rayon, tangences et arrêts source prouvés ; aucun arrondi propagé implicitement | À développer avec contrat local de raccords |

Source : [ancienne feuille de route](../../docs/ROADMAP-avant-fabboard.md). Les validations historiques sont conservées ; aucun essai nouveau n’est déduit de la migration.

## Poursuite demandée le 17 novembre

Première qualification locale lancée dans R-062 ; les coins à trois branches restent ici.


## Poursuite globale autorisée

L’utilisateur confirme que l’objectif est de réaliser l’ensemble C1b2, C2, D, E, F et G et autorise la poursuite du chantier. Les tranches déjà développées restent à tester ; cet accord ne remplace pas leur acceptation ni les preuves géométriques. Les travaux sont décomposés en extensions bornées, corpus indépendants et vérifications natives. Les changements structurants encore indéfinis seront présentés lorsqu’un contrat concret est établi ; l’accord sur les perçages sécants A-045 reste acquis.


## Intégration des tranches

Le contrat local est approuvé par A-068/ADR-0036. R-081 intègre le coin à trois cylindres et un octant sphérique sur bloc orthogonal ; qualification globale V-1203 réussie ; essais A-075/A-076 ouverts.

R-097 — Reconstruire une paroi cylindrique concave tangente et ses deux bords elliptiques dans une poche borgne. Voir V-1225.

R-101 — Reconstruire deux congés de poche de même rayon partageant une arête elliptique sur la bissectrice du coin. Voir V-1231.
