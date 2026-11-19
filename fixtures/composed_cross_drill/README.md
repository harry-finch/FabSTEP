# Congé longitudinal et perçages sécants

Les STL proviennent du générateur indépendant `scripts/qualify-r114-composition-freecad.py`, qui redessine les solides avec FreeCAD, écrit et relit le STEP puis le maille. Pour les régénérer, lancer ce script avec le Python de FreeCAD dans un répertoire temporaire.

Les deux fichiers `disjoint` décrivent un bloc de 24 × 20 × 18 mm, avec deux trous traversants centrés de rayons 6 et 3 mm, orthogonaux, et un congé extérieur de 1,5 mm sur toute la longueur. Les déflexions angulaires de 0,20 et 0,10 rad donnent 982 et 1 878 triangles. Le contrôle du résultat attend six plans, trois parois concaves et un quart de cylindre convexe. La quadrature de l’intersection des trous donne un volume de matière de 5 733,46608 mm³ après retrait du congé.

Dans `common-bore-coarse.stl`, deux trous transversaux rejoignent le même trou longitudinal. Leurs revendications se recouvrent sur 524 triangles. Le test contrôle le refus de la composition.
