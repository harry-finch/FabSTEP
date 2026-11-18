---
id: R-034
titre: "D — congés partiels constants"
resume: "Coin à trois arrondis et congé de poche à deux butées intégrés. Restent les autres arrêts et réseaux de congés."
statut: en-cours
jalon: post-v1
cree: 2026-11-13
maj: 2026-11-18
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

## État consolidé — 18 novembre 2026

R-081 intègre le coin orthogonal à trois arrondis ; R-062 qualifie des coins verticaux de bac. R-086 qualifie un congé intérieur à deux butées droites et des extrémités de subdivisions différentes, y compris sur un congé extérieur. Reste : généralisation des configurations de congés partiels et de leurs combinaisons. Le domaine global D n’est pas terminé.

L’utilisateur a autorisé la réalisation de l’ensemble. Le statut « en cours » représente ce chantier global partiellement intégré ; il ne signifie pas que tous ses domaines sont déjà disponibles. Les essais manuels ouverts ne bloquent pas les travaux indépendants restants.

## Historique et décisions

| D — congés partiels constants | Une ou deux arêtes droites arrondies, puis coin à trois branches | Supports, rayon, tangences et arrêts source prouvés ; aucun arrondi propagé implicitement | À développer avec contrat local de raccords |

Source : [ancienne feuille de route](../../docs/ROADMAP-avant-fabboard.md). Les validations historiques sont conservées ; aucun essai nouveau n’est déduit de la migration.

## Poursuite demandée le 17 novembre

Première qualification locale lancée dans R-062 ; les coins à trois branches restent ici.


## Poursuite globale autorisée

L’utilisateur confirme que l’objectif est de réaliser l’ensemble C1b2, C2, D, E, F et G et autorise la poursuite du chantier. Les tranches déjà développées restent à tester ; cet accord ne remplace pas leur acceptation ni les preuves géométriques. Les travaux sont décomposés en extensions bornées, corpus indépendants et vérifications natives. Les changements structurants encore indéfinis seront présentés lorsqu’un contrat concret est établi ; l’accord sur les perçages sécants A-045 reste acquis.


## Intégration des tranches

Le contrat local est approuvé par A-068/ADR-0036. R-081 intègre le coin à trois cylindres et un octant sphérique sur bloc orthogonal ; qualification globale V-1203 réussie ; essais A-075/A-076 ouverts.
