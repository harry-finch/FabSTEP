# ADR-0031 — Chaînes coniques de signe libre avec portées

Date : 17 novembre 2026. Statut : décision retenue dans la poursuite géométrique demandée par l’utilisateur ; validation dans R-032.

## Contexte

C1a admet trois à huit cônes alternés séparés par des portées cylindriques résolues. Les mêmes preuves de surface et de séparation sont utilisables lorsque les pentes successives ont le même signe. En revanche, retirer les portées exige une nouvelle preuve de segmentation ; ce cas reste séparé dans R-059.

## Décision

Admettre deux à huit cônes complets de signe libre, avec une portée cylindrique entre chaque paire. Chaque portion du nouveau parcours, y compris les éventuels cylindres terminaux, possède deux anneaux source intérieurs distincts, dans les intervalles normalisés `[1/4,1/2[` et `]1/2,3/4]`, séparés de plus de `4ε`.

Conserver la séparation d’incertitude existante entre cône et portée, avec sa marge quatre, et toutes les preuves d’affinité, de paroi, d’orientation et de couverture. Le cas historique de deux chanfreins opposés garde son propre contrat sans nouveaux témoins imposés rétroactivement. Les bandes coniques consécutives sans portée restent fusionnées selon une loi affine unique, sans découpage opportuniste.

Les garde-fous sont partagés entre Primitives et Surfaces ; Auto réutilise le parcours local de Surfaces. Aucun changement de DTO, d’ABI, de tolérance ou de dépendance. Les compteurs de travail, plafonds, contrôles source, réimport solide, volume, dimensions et audit restent obligatoires. Une interprétation analytique du maillage ne prétend pas retrouver l’intention de conception.

## Qualification

Corpus indépendant de chaînes montantes, descendantes et mixtes, transformations et placements, avec volumes et comptes attendus déterminés avant conversion. Cas négatifs : absence de témoins, méridienne courbe, portée ambiguë tardive, budget et plafond dépassés. Les résultats exécutés sont consignés dans VALIDATION ; aucune qualification graphique ou Fusion n’est déduite des tests natifs.
