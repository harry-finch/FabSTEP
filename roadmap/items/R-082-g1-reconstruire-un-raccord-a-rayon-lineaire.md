---
id: R-082
titre: "G1 — reconstruire un raccord à rayon linéaire"
resume: "Reconnaître l’enveloppe conique inclinée à extrémités elliptiques d’un arrondi linéaire, sans la confondre avec des sections circulaires empilées."
statut: specifiee
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-17
livree:
---
## But

Reconstruire un premier arrondi à rayon variable exact : une loi linéaire positive sur une arête droite entre deux plans orthogonaux, avec extrémités planes présentes. Première intégration source du domaine G approuvé dans ADR-0036 et A-068.

## Inclus

- Une arête convexe d’un bloc, enveloppe régulière à rayon linéaire entre deux supports plans orthogonaux et deux plans terminaux horizontaux dans le repère prouvé.
- Rayons terminaux et pente retrouvés depuis quatre extrémités et témoins intérieurs source, interprétation unique, pente séparée de zéro et des coupes paraboliques.
- Portion de cône circulaire inclinée, deux génératrices tangentes et deux arcs elliptiques partagés avec les plans ; rayon constant conservé dans son parcours existant.
- Couverture du domaine tronqué, projection vectorielle et borne continue par triangle ; distinction mesurable avec un faux empilement de sections circulaires.
- Auto et Surfaces, STEP réimporté, volume/dimensions/audit/provenance ; modèle additif conservant les anciens cônes partiels circulaires.

## Hors périmètre

Lois non linéaires, rayons nuls, coupes paraboliques/hyperboliques, supports non orthogonaux, plusieurs congés variables raccordés, surfaces libres ou approximation implicite. Les autres parties de G restent R-037.

## Dépendances

R-079, ADR-0036 et A-068 approuvés ; intégrer les changements de frontière core/CAO après D2 ou via écritures exclusivement coordonnées.

## Critères d'acceptation

- Corpus indépendant pour croissance/décroissance, transformations et retessellisations ; faux empilement circulaire, loi non linéaire, frontière altérée et budget trop strict refusés.
- Six plans et un cône incliné, un solide fermé réimporté, deux ellipses et deux génératrices partagées, incidences opposées et tangences vérifiées.
- Chaque triangle source possède un propriétaire unique ; couverture orientée du domaine complet et erreurs des frontières comptées.
- Volume analytique et dimensions cohérents, aucun seuil relâché ; guide non pris en charge refusé explicitement et anciens formats inchangés.
- Revue indépendante, tests core/natifs/provenance et régressions réussis.

## Vérifications de l'utilisateur

- Examiner les variantes de rayon linéaire dans FabSTEP et leurs faces terminales.
- Réimporter leurs STEP dans Fusion et vérifier solide, volume et raccords elliptiques.
