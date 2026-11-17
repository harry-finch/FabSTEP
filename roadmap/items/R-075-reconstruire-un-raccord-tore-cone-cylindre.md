---
id: R-075
titre: "Reconstruire un raccord tore–cône–cylindre prouvé"
resume: "Première extension issue de R-074 : reconstruire la zone de 64 080 triangles par trois surfaces, avec raccord tangent prouvé et bandes à deux anneaux. Contrat de données inchangé ; nouvelle règle de preuve à valider, sans promesse de convertir Flansch entier."
statut: idee
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-17
livree:
---

## But

Reconstruire une chaîne plane–tore–cône–cylindre–plan dont chaque frontière existe dans la source. Première cible : la zone de 64 080 triangles mesurée dans R-074, sans chercher immédiatement les trois autres chaînes ni la conversion entière de Flansch.

## Inclus

- Un arc de tore tangent au plan terminal et au cône voisin, puis un cylindre, sur un tour complet ; cas mesuré et son symétrique.
- Décomposition unique et bornée entre anneaux source ; calcul conjoint des paramètres et du raccord, sans rayon nominal imposé.
- Bandes conique et cylindrique à deux anneaux admises uniquement avec preuve complète de topologie, couverture UV, orientation, raccords et écart sur les triangles ; aucun témoin intérieur inventé.
- Orientation des anneaux et dépliage des angles adaptés à la branche du tore, sans modifier l’ordre des portions droites.
- Reconstruction automatique et mode Surfaces, avec provenance exhaustive, volume/dimensions, réimport et audit ; guidage Tore existant conservé.

## Hors périmètre

Deux cônes dans la chaîne, arcs de 100° du couple restant, profils libres, décimation, relèvement des limites, nouveau format public ou ABI. Aucun affaiblissement global des gardes historiques ; aucune promesse de STEP complet pour Flansch.

## Dépendances

R-074 et décision utilisateur sur la preuve des bandes à deux anneaux. MixedTorus et la frontière CAO existants suffisent en représentation ; les règles de reconnaissance et de preuve doivent être étendues.

## Critères d'acceptation

- Corpus indépendant de la donnée privée : cas nominal, symétrie, rotation, branche opposée et placements.
- Refus des transitions ambiguës/non tangentes, anneaux absents, triangles traversant une frontière, maillage perturbé hors tolérance et budgets insuffisants.
- Anneaux réellement partagés entre tore, cône, cylindre et plans ; erreurs de déplacement comptées, orientation et borne continue vérifiées.
- Source et résultat nativement valides ; STEP réimporté, volume/dimensions, types de surfaces, provenance et fidélité contrôlés.
- Chemins historiques inchangés et suites de non-régression adaptées réussies ; limites restantes de Flansch affichées honnêtement.

## Vérifications de l'utilisateur

- Valider cette portée et la preuve des bandes à deux anneaux avant implémentation.
- Après développement, examiner un résultat de référence dans l’application et dans Fusion.

## Arbitrage

Recommandation : commencer par cette seule chaîne (trois surfaces attendues) et réutiliser ses preuves ensuite. Avantage : première extension bornée sans nouvelle représentation de surface. Risque : toute erreur d’orientation ou de raccord peut invalider le solide ; les tests négatifs et le réimport sont obligatoires. Alternative : traiter les quatre zones ensemble, plus large et plus risqué car elle ajoute deux cônes et des arcs de 100° avec dépliage angulaire. La validation native du maillage dense et les 231 584 plans hors des quatre zones restent des obstacles indépendants.
