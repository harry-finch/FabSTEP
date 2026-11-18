# ADR-0041 — Cylindre partiel arrêté par un plan oblique

**Statut** — Proposition soumise dans A-087. Domaine de R-087.

## Géométrie attendue

Le congé de fond est tangent à deux plans. Ses deux génératrices atteignent la butée oblique à des distances différentes. Les faces de la poche doivent se partager quatre arêtes réelles. Sur le témoin de FreeCAD, ce sont deux segments de 23 et 24,875 mm, un quart de cercle de rayon 2,5 mm et un quart d’ellipse de demi-axes 3,125 et 2,5 mm. La forme et le volume de référence sont vérifiés dans V-1212.

## Choix soumis

Étendre la représentation du cylindre partiel pour référencer un arc de cercle d’un côté et un arc d’ellipse de l’autre. Le cercle fournit l’axe et le rayon. Les extrémités des deux arcs identifient les deux génératrices, en vérifiant leur appariement et l’orientation du contour. Les types existants `CircularArc`, `EllipticArc` et les segments des boucles planes portent déjà les paramètres des deux courbes. Le transfert devra conserver l’identité de chaque arête sur ses deux faces. Les anciens modèles à deux arcs de cercle doivent continuer à se lire. La famille présentée dans l’inspecteur reste « Cylindre ».

L’autre option est la conversion facettisée actuelle. Elle fournit un solide fermé mais découpe le congé en plusieurs faces et la butée en plusieurs segments. Le chemin réservé aux cônes à rayon variable impose un demi-angle positif et deux ellipses. Il ne représente pas ce cylindre.

## Conditions d’intégration dans Auto

- Déduire l’axe et le cercle des témoins source. Retrouver le plan oblique, intersecter ce plan avec le cylindre et vérifier ses échantillons, ses deux coins et les deux tangences. Le plan oblique doit couper toutes les génératrices sur l’intervalle certifié.
- Dans le repère du cylindre, la distance axiale entre les deux butées varie comme une constante plus deux termes en sinus et cosinus. En vérifier la positivité sur tout l’arc, y compris aux extrema intérieurs. Vérifier une couverture orientée unique de la paroi et ses incidences source. Comptabiliser les écarts des sommets et la flèche entre deux échantillons, y compris sur l’ellipse.
- Calculer le volume avec la hauteur variable. Contrôler la provenance exhaustive, le solide, ses dimensions, son volume et ses quatre bords après réimport STEP. Les essais comparent les profils des deux butées et les sondes de matière sur une pièce CAO indépendante.
- Préserver les refus liés à la fidélité et aux plafonds de travail. Les intersections de plusieurs congés et les cylindres terminés par deux plans obliques demandent une qualification distincte.
