---
id: R-037
titre: "G — congés variables"
resume: "Premier raccord à rayon linéaire intégré ; restent les autres configurations et raccords variables multiples."
statut: en-cours
jalon: post-v1
cree: 2026-11-13
maj: 2026-11-18
livree:
---

## But

Reconstruire des congés à rayon variable à partir de limites et de supports source démontrés.

## Inclus

Première tranche à rayon linéaire positif entre deux plans orthogonaux, puis qualification des autres configurations et raccords multiples.

## Hors périmètre

Lois générales ou raccords multiples non encore qualifiés, rayons nuls, singularités et approximations implicites.

## Dépendances

A-068/ADR-0036, R-079 et R-082 ; nouveaux domaines à qualifier avant activation.

## Critères d'acceptation

Loi identifiable, enveloppe régulière, couverture et écart continus, volume et STEP réimporté contrôlés.

## Vérifications de l'utilisateur

Les essais de chaque tranche sont portés par ses sous-jalons et leurs actions FabBoard ; ne pas les dupliquer au niveau de ce chantier global.

## État consolidé — 18 novembre 2026

R-082 intégré et à tester : raccord convexe linéaire borné avec deux ellipses exactes. Reste : autres configurations et raccords variables multiples. La première tranche n’est pas assimilée à une reconnaissance générale.

L’utilisateur a autorisé la réalisation de l’ensemble. Le statut « en cours » représente ce chantier global partiellement intégré ; il ne signifie pas que tous ses domaines sont déjà disponibles. Les essais manuels ouverts ne bloquent pas les travaux indépendants restants.

## Historique et décisions

| G — congés variables | Arête droite entre deux plans, rayon linéaire positif, arrêts source | Loi identifiable, enveloppe régulière, bornes continues, volume et réimport ; stabilité aux tessellations | Recherche conditionnelle ; lois générales et raccords multiples différés |

Source : [ancienne feuille de route](../../docs/ROADMAP-avant-fabboard.md). Les validations historiques sont conservées ; aucun essai nouveau n’est déduit de la migration.


## Poursuite globale autorisée

L’utilisateur confirme que l’objectif est de réaliser l’ensemble C1b2, C2, D, E, F et G et autorise la poursuite du chantier. Les tranches déjà développées restent à tester ; cet accord ne remplace pas leur acceptation ni les preuves géométriques. Les travaux sont décomposés en extensions bornées, corpus indépendants et vérifications natives. Les changements structurants encore indéfinis seront présentés lorsqu’un contrat concret est établi ; l’accord sur les perçages sécants A-045 reste acquis.


Première tranche intégrée : [R-082](R-082-g1-reconstruire-un-raccord-a-rayon-lineaire.md), raccord convexe linéaire borné, contrôles V-1204 et essais A-077/A-078. Les lois générales et raccords multiples restent distincts ; le jalon global G n’est pas déclaré livré.
