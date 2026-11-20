---
id: R-123
titre: "Fusionner les faces planes voisines des raccords toriques"
resume: "F1 · Mesurer puis réduire les faces résiduelles et les chaînes de petits raccords sur deux pièces publiques."
statut: specifiee
jalon: post-v1
cree: 2026-11-20
maj: 2026-11-20
livree:
---
## But

Une couronne torique peut être correctement reconstruite tout en restant entourée de centaines de petites faces difficiles à sélectionner en CAO. Le premier domaine retenu est une plage plane contiguë au tore.

## Inclus

- Inventorier les triangles résiduels coplanaires, les chaînes de colliers et les groupes OCCT coûteux qu’ils alimentent.
- Fusionner une plage seulement si les boucles, perçages, plis et frontières partagées sont conservés. Remplacer une chaîne de colliers uniquement si son bord commun avec le tore est vérifié.
- Comparer sur les mêmes entrées les faces finales, les chaînes supprimées, la taille STEP et le coût du contrôle avant et après transfert.

## Hors périmètre

Les voisins cylindriques ou coniques relèvent de R-125. Aucun facteur d’accélération sur Flansch n’est présupposé.

## Dépendances

Les deux témoins publics initiaux de R-122 et le constructeur torique de R-085. Le rejeu privé de R-085 ne conditionne pas ce lot.

## Critères d'acceptation

- Convertir depuis le STL en Auto une petite bride et une seconde pièce de forme différente, à deux densités. Réduire les faces sur les deux, montrer au moins une chaîne entièrement supprimée et identifier les groupes affectés.
- Réimporter un solide fermé avec les mêmes perçages, volume et dimensions dans les tolérances existantes. Vérifier les deux directions de fidélité et une attribution unique de chaque triangle.
- Une plage légèrement pliée et un raccord dont la frontière est ambiguë gardent leur représentation précédente. Si la seule réduction porte sur les triangles plans, consigner séparément ce résultat et garder ouvert l’objectif sur les colliers.

## Vérifications de l'utilisateur

Ouvrir la pièce publique dans Fusion et sélectionner d’un clic la plage plane voisine du congé.
