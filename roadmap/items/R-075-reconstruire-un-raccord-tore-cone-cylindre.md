---
id: R-075
titre: "Reconstruire un raccord tore–cône–cylindre prouvé"
resume: "Première extension issue de R-074 : reconstruire la zone de 64 080 triangles par trois surfaces, avec raccord tangent prouvé et bandes à deux anneaux. Contrat de données inchangé ; règle de preuve approuvée pour implémentation, sans promesse de convertir Flansch entier."
statut: a-tester
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

- Portée et preuve des bandes à deux anneaux approuvées par « c’est parti pour R-075 » (A-064).
- Après développement, examiner un résultat de référence dans l’application et dans Fusion.

## Arbitrage

Recommandation : commencer par cette seule chaîne (trois surfaces attendues) et réutiliser ses preuves ensuite. Avantage : première extension bornée sans nouvelle représentation de surface. Risque : toute erreur d’orientation ou de raccord peut invalider le solide ; les tests négatifs et le réimport sont obligatoires. Alternative : traiter les quatre zones ensemble, plus large et plus risqué car elle ajoute deux cônes et des arcs de 100° avec dépliage angulaire. La validation native du maillage dense et les 231 584 plans hors des quatre zones restent des obstacles indépendants.

## Résultat intégré

Détecteur borné dans `rounded/torus_cone.rs`, partage du budget existant, unique découpage sur des anneaux source, tangence calculée conjointement et contrôle continu par triangle. L’ordre des anneaux du tore est adapté à la branche et au reflet. La restauration des mesures évite de réutiliser les ajustements d’un essai historique refusé.

Le corpus synthétique couvre aussi une cavité traversante et une rotation oblique par placement 3MF. Le STL arbitrairement tourné puis arrondi en f32 reste un refus de coplanarité explicite ; aucun garde n’est relâché. La revue indépendante ne relève pas de défaut bloquant.

Sur Flansch, les 64 080 triangles sont répartis en un tore (62 640), un cône (720) et un cylindre (720), sans débordement. Le helper propre obtient 439 738 faces contre 501 912 auparavant, puis refuse toujours le plafond de 100 000 avant validation native source : aucun STEP. Le parcours des composantes retire des auto-visites inutiles pour conserver la reconnaissance dans le budget, avec équivalence testée. Preuves et limites complètes : V-1199 dans `docs/VALIDATION.md`.
