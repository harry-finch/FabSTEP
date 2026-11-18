# ADR-0042 — Arête commune de deux congés de fond adjacents

**Acceptée le 18 novembre 2026.** L’accord A-089 autorise l’extension du contrat entre le moteur de reconnaissance et la CAO. Le domaine de R-089 couvre deux quarts de cylindre intérieurs de même rayon, d’axes perpendiculaires, sur deux côtés adjacents d’une poche à fond plan.

## Géométrie observée

La pièce de référence a onze plans et deux cylindres. Les cylindres s’arrêtent l’un sur l’autre le long d’un quart d’ellipse dans le plan bissecteur. Pour le rayon `r`, les demi-axes valent `r × √2` et `r`. Chaque cylindre rejoint aussi le fond, une paroi latérale et une butée droite. Leurs deux arcs d’arrêt circulaires se trouvent sur des faces planes distinctes. Les pièces, les mesures et les limites du contrôle FreeCAD figurent dans V-1215.

## Décision

Autoriser une même arête elliptique en trois dimensions à border deux cylindres partiels. Chaque face devra en porter un tracé paramétrique compatible avec son propre axe et sa propre orientation. Le contrat actuel (ADR-0041) valide une ellipse partagée par un cylindre et un plan. Réutiliser les types de courbes existants et garder les anciens modèles lisibles. L’alternative est de conserver le solide facettisé actuel dans ce coin.

## Conditions d’intégration

- Prouver sur les triangles les deux axes, le rayon commun, les arcs d’arrêt, les quatre rails et une chaîne partagée de sommets sur la branche attendue. Chaque triangle appartient à une seule face. Un rayon différent, une discontinuité ou un coin déformé doit empêcher cette interprétation.
- Vérifier la même arête dans les deux paramétrisations sur tout l’intervalle. Vérifier que le solide réimporté conserve la courbe commune, les deux cercles d’arrêt, la concavité et les voisins. Conserver les plafonds de travail et les tolérances du projet.
- Comparer la matière et le vide des deux côtés de la jonction, le volume et les dimensions au modèle attendu. Couvrir des tessellations indépendantes, une pièce tournée et les cas négatifs. Avant intégration, vérifier l’attribution source en Auto et le réimport de plusieurs corps distincts.

## Domaine intégré

Chaque cylindre partiel conserve son indice de cercle. Tous deux portent `elliptic_end = true` et référencent la même ellipse, parcourue en sens inverse sur la seconde paroi. Les coordonnées de ses extrémités déterminent les rails associés. Les deux faces restent des cylindres dans l’inspecteur. Les fichiers décrits par ADR-0041 restent lisibles. Cette nouvelle combinaison requiert un lecteur qui connaît le partage du bord entre deux cylindres.

L’implémentation réutilise la sélection de onze supports plans sur un corps de 24 à 4 096 triangles. Elle exige deux supports latéraux orthogonaux, une butée droite par congé et un rayon commun résolu. La vérification de hauteur utilise les extrema de la fonction axiale sur le quart d’arc effectif. Elle accepte notamment une longueur de poche de 3,75 mm avec un rayon de 2 mm. La borne de fidélité, l’attribution des triangles et les tests après réimport figurent dans V-1216.
