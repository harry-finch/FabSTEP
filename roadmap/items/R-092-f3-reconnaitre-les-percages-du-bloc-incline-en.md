---
id: R-092
titre: "F3 — reconnaître les perçages du bloc incliné en STL binaire"
resume: "Deux poses et deux densités de STL binaire inclinés donnent six plans et trois cylindres aux réglages par défaut. Vérification Fusion ouverte."
statut: a-tester
jalon: post-v1
cree: 2026-11-18
maj: 2026-11-18
livree:
---
## But

Convertir en mode automatique le STL binaire incliné de la pièce à deux perçages qui échouait dans V-1219.

## Inclus

Identifier la cause des contours artificiellement pincés lors de la segmentation planaire. Prouver les supports sur les points source, puis contrôler les rayons, la pose et le volume du solide reconstruit. Réutiliser les contrôles existants de projection et de réimport.

## Hors périmètre

Les deux axes de perçage restent orthogonaux et centrés dans le bloc. Les pièces dont les axes de perçage sont obliques entre eux relèvent de R-036.

## Dépendances

R-083 et R-091 décrivent le domaine géométrique. Le maillage et la référence CAO proviennent du générateur FreeCAD de R-091.

## Critères d'acceptation

Aux réglages par défaut, retrouver six faces planes et trois faces cylindriques en réimportant le STEP. Contrôler dans FreeCAD la pose, les deux coutures, la matière et le volume au seuil de V-1219. Couvrir la provenance, les vrais pincements et les cas hors tolérance.

## Vérifications de l'utilisateur

- Reconstruire le STL incliné en mode Auto et contrôler dans Fusion les diamètres de 6 et 10 mm ainsi que la position des ouvertures.

## Résultats

Deux poses à deux résolutions passent la relecture FreeCAD avec les réglages initiaux. Les quatre STEP comportent six plans et trois parois cylindriques avec leurs deux courbes communes. La plaque présentant un pli de 0,0000016 mm conserve ses deux pans en mode Plans. Les mesures et la commande de reproduction figurent dans V-1220. L’essai dans Fusion est suivi dans A-095.
