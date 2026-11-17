# ADR-0035 — Raccord tore–cône–cylindre à frontières source

Date : 17 novembre 2026. Statut : portée approuvée par l’utilisateur pour R-075 ; décision retenue, implémentation et validation automatisée réussies (V-1199) ; qualification utilisateur ouverte.

## Contexte

R-074 mesure dans Flansch une chaîne plan–tore–cône–cylindre–plan dont les bandes droites n’ont que deux anneaux. Le modèle `MixedTorus` sait déjà représenter le raccord ; un test natif indépendant valide le nominal et son reflet. Le reconnaisseur historique impose des raccords quart de cercle cylindre–plan et ne sait pas prouver cette chaîne. Supprimer son refus ou ajouter des témoins inexistants ne résoudrait ni l’orientation ni les frontières partagées.

## Décision

Ajouter une reconnaissance bornée de cette seule chaîne en Auto et Surfaces, pour un tour complet avec des anneaux réellement présents. Les chemins historiques restent prioritaires. Le cône, sa portée et le plan terminal déterminent conjointement le cercle tangent : pas de rayon nominal imposé ni d’ajustements indépendants laissant une couture entre faces. L’unicité et la résolution du découpage sont contrôlées avant acceptation.

Les bandes droites à deux anneaux utilisent la topologie, la couverture paramétrique orientée de degré un et la borne continue sur chaque triangle. Cette admission reste propre à la nouvelle chaîne ; elle ne supprime pas les exigences de témoins des autres profils. Tous les déplacements de frontières et de sommets entrent dans les résidus et dans le budget géométrique.

L’ordre axial des anneaux des portions droites est conservé. Celui du tore suit les angles méridiens croissants, même sur la branche de rayon radial inférieur au rayon majeur où les deux ordres s’opposent. Les transformations et reflets doivent préserver le solide, la provenance et le signe d’orientation démontré.

Aucun changement de DTO, ABI, dépendance ou plafond. Les erreurs de ressources demeurent bloquantes ; aucune preuve ancienne ne devient valable pour une géométrie modifiée.

## Vérification et limites

Corpus synthétique indépendant avec nominal, reflet axial, rotation, branche opposée, perturbation bornée et placements 3MF. Refus des défauts de tangence, d’anneaux ou de couverture, des perturbations hors tolérance et des budgets insuffisants. Tests core puis validation source native, construction/réimport, volume, dimensions, familles, provenance et audit bidirectionnel.

Le guidage Tore avec voisin conique reste hors périmètre. Les chaînes à deux cônes et les arcs de 100° de Flansch demandent des extensions distinctes. Les plans restants et le coût de validation native de la source dense empêchent de promettre la conversion de Flansch entier.
