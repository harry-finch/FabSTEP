---
id: R-059
titre: "C1b2 — chaînes coniques sans portée intermédiaire"
resume: "Deux à huit cônes adjacents ou mêlés à des portées sont couverts par R-060 et R-078. Les essais de la chaîne complète sont suivis en A-073 et A-074."
statut: a-tester
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-18
livree:
---

Partie conservée du périmètre initial de R-032. Les classificateurs actuels fusionnent les bandes coniques consécutives selon une même loi affine ; identifier plusieurs cônes sans portée exige de prouver des frontières source complètes, des témoins intérieurs et une séparation des intervalles de pente. Aucune levée directe du garde ni découpe d’une méridienne lisse au niveau de chaque anneau. Cette recherche reste distincte de la tranche avec portées de R-032.

## Poursuite demandée le 17 novembre

Première tranche bornée lancée dans R-060 : deux cônes sans portée. Les chaînes plus longues et mélangées restent ici.


## Poursuite globale autorisée

L’utilisateur confirme que l’objectif est de réaliser l’ensemble C1b2, C2, D, E, F et G et autorise la poursuite du chantier. Les tranches déjà développées restent à tester ; cet accord ne remplace pas leur acceptation ni les preuves géométriques. Les travaux sont décomposés en extensions bornées, corpus indépendants et vérifications natives. Les changements structurants encore indéfinis seront présentés lorsqu’un contrat concret est établi ; l’accord sur les perçages sécants A-045 reste acquis.


## Intégration des tranches

R-078 étend les chaînes adjacentes ou mêlées à des portées jusqu’à huit cônes, avec témoins intérieurs et découpage unique. Validation automatique V-1201 réussie, essais A-073/A-074 ouverts ; les profils hors domaine gardent le repli Auto historique.

## Qualification du domaine prévu

La suite native `fabstep-app --test adjacent_chains` vérifie en réimport les chaînes de trois à huit cônes et leur combinaison avec des portées. Elle contrôle aussi la provenance et le refus des ruptures ambiguës. R-060 couvre deux cônes. Les essais utilisateur des chaînes longues et mixtes sont déjà ouverts en A-073 et A-074. Le plafond de huit cônes et l’exigence de témoins à l’intérieur de chaque bande restent applicables.
