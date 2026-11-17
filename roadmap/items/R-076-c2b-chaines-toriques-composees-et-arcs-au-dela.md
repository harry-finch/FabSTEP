---
id: R-076
titre: "C2b — chaînes toriques composées et arcs au-delà de 90°"
resume: "Reconstruire les trois autres zones courbes mesurées sur Flansch : raccord à deux cônes et arcs de 100°, avec frontières source et preuves continues."
statut: en-cours
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-17
livree:
---
## But

Poursuivre C2 après R-075 en reconstruisant les trois autres grandes zones courbes identifiées sur Flansch. L’accord utilisateur sur l’ensemble C1b2, C2, D, E, F et G autorise cette tranche proposée immédiatement après R-075 ; aucune conversion entière de Flansch n’est présumée.

## Inclus

- Chaîne plan–quart de tore–cylindre–cône–cylindre–cône–plan, avec transitions et longueurs résolues.
- Chaîne plan–tore–cône–plan dont l’arc méridien dépasse 90° et reste axialement monotone, notamment les arcs de 100° mesurés.
- Décomposition unique sur des anneaux source, bandes droites à deux anneaux avec couverture topologique et borne continue ; tangence commune calculée depuis les mesures.
- Reflets, branches opposées, rotations et placements ; modes Auto et Surfaces, provenance exacte et réimport natif.
- Mesure bornée des attributions restantes sur Flansch sans modifier les plafonds.

## Hors périmètre

Méridiennes libres, retours axiaux, pôles, chaînes arbitraires, nouveau DTO ou ABI, guidage torique étendu et décimation. Les autres composantes C1b2/D/E/F/G conservent leur suivi séparé. Pas de seuil relevé ni de fermeture inventée.

## Dépendances

R-074, R-075 et leurs preuves. Accord global de poursuite reçu ; modèle MixedTorus existant conservé.

## Critères d'acceptation

- Corpus synthétique indépendant reproductible ; volumes et dimensions analytiques, surfaces attendues et partition source vérifiés.
- Refus des frontières absentes/traversées, transitions ambiguës, tangences incompatibles, déformations et budgets dépassés.
- Arcs passant par 90° et la couture angulaire traités sans inversion de domaine ; anneaux partagés et borne composée contrôlés.
- Compatibilité des parcours historiques et R-075, revue indépendante et tests ciblés/globaux réussis.
- Flansch mesuré honnêtement : compte final et refus éventuel, aucun export partiel.

## Vérifications de l'utilisateur

- Examiner les nouveaux raccords du corpus dans FabSTEP et exporter leurs STEP.
- Réimporter les STEP de référence dans Fusion et vérifier les raccords, volumes et dimensions.
