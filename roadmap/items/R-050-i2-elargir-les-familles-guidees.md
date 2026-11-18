---
id: R-050
titre: "I2 — élargir les familles guidées"
resume: "Après Plan (R-056), ajouter le raccord torique guidé puis étudier les sphères."
statut: a-tester
jalon: atelier
cree: 2026-11-13
maj: 2026-11-18
livree:
---
## But

Choisir une famille sur des faces STEP pour corriger la région source et contrôler la pièce reconstruite.

## Inclus

Plan, cylindre, cône, tore avec portée et sphère vérifiés dans leurs domaines. Pour Sphère, sélectionner une face d’une sphère complète, d’une calotte ou d’une zone sphérique terminée sur des plans. Le bouton Essayer relance la validation du document et de sa provenance. Le rapport conserve les triangles réellement reconstruits.

## Hors périmètre

Sélection traversant plusieurs familles, voisin courbe dont il faudrait modifier le raccord, octant d’un congé à trois branches et surface libre.

## Dépendances

R-056 et R-057 pour la préparation, les indications et leur recalcul. V-1230 pour la qualification de la sphère.

## Critères d'acceptation

- Sur les documents Plans et Auto, l’essai conserve les limites source et les propriétaires de triangles.
- L’indication peut être recalculée puis retirée.
- Un cylindre ou une sélection mixte donne un refus de famille.

## Vérifications de l'utilisateur

- Ouvrir le témoin de demi-sphère, sélectionner une face du dôme dans le STEP, choisir Sphère et utiliser Essayer. Vérifier le dôme, le disque et la possibilité de retirer l’indication.

## Historique de la spécification

**Note utilisateur du 10 novembre — choix explicite de la forme à reconnaître.** Prévoir la sélection d’un groupe de facettes du STEP, puis une action contextuelle permettant d’indiquer la famille géométrique à rechercher pour corriger cette zone : cylindre, cône, sphère ou autre famille effectivement prise en charge. Cette extension du lot I2 complète le bouton actuel « Essayer un cylindre » ; I2c ajoute les troncs de cône complets entre deux contours plans source. Les autres familles restent à développer. L’hypothèse doit être vérifiée sur la zone et ses raccords, avec aperçu du résultat, écart tracé et refus explicite si les preuves sont insuffisantes. La sélection ne force jamais une primitive. Ce chantier reprend après DXF2 avec I2c.

Source : [ancienne feuille de route](../../docs/ROADMAP-avant-fabboard.md). Les validations historiques sont conservées ; aucun essai nouveau n’est déduit de la migration.


## Suite convenue le 15 novembre 2026

Priorité demandée : Plan (lot R-056), puis Tore. Pour Tore, étudier en premier les raccords toriques déjà reconnus automatiquement ; la sélection guidée devra conserver les frontières et la provenance, contrôler les voisins et revalider le solide entier. Ne pas afficher un bouton actif avant disponibilité réelle. Les sphères restent une extension ultérieure.

### Qualification technique du prochain lot Tore

Le raccord existant (`rounded.rs`) est un quart de méridienne sur un tour complet, entre cylindre et plan réels. Sa preuve impose une frontière circulaire commune et peut ajuster la portée cylindrique : on ne peut pas garder cette portée facettisée tout en reconstruisant le seul congé. Le guidage Cylindre actuel ne permet pas de préparer séparément cette portée, faute de deux supports plans.

Premier périmètre utile proposé : un congé et sa portée cylindrique, délimités par les plans source. Exclure secteurs angulaires, doubles congés, tore–tore, voisins coniques et surfaces libres. Le tore entier isolé est un cas différent, moins utile pour les corrections locales demandées.

Étapes : preuve locale restreinte sans reconnaissance des zones extérieures ; provenance native additive pour RoundedModel et jetons réels après couture/réimport ; session avec retrait/recalcul, rapport et surlignage de toute la zone modifiée. Le parcours `build_rounded_step` actuel n’expose pas de provenance suivie.

Arbitrage demandé : sélection explicite congé+cylindre (recommandée pour un premier lot prévisible), ou sélection du congé et proposition d’extension à confirmer. Pas de modification du voisin sans portée explicitement acceptée.


### Intégration du 16 novembre

La famille Tore et son groupe de sélection avec portée cylindrique sont développés dans R-057, au sein de la reconstruction automatique et du guidage local. Les chaînes avec voisins coniques/sphériques sont refusées dans ce lot ; les raccords libres et autres familles restent à spécifier ici. L’arbitrage A-034 est remplacé par le parcours global approuvé le 16 novembre.
