---
id: R-113
titre: "Qualifier les contours obliques et les parois de révolution interrompues"
resume: "La lèvre torique est qualifiée à deux densités. Sur trois petits STL d’arbres percés, le prototype mesure deux cylindres et produit un STEP valide à quatre faces."
statut: en-cours
jalon: v1.0
cree: 2026-11-19
maj: 2026-11-19
livree:
---

## But

Qualifier le bord où un plan coupe une lèvre torique en biais et celui où un perçage traverse un arbre cylindrique. Sur ces pièces, les facettes seules masquent les deux surfaces mécaniques qui se rejoignent.

## Inclus

- Utiliser les deux solides indépendants de V-1260, chacun à deux densités. Les supports nominaux sont connus pour cette qualification.
- Retrouver les propriétaires des triangles à partir du STL et les comparer aux faces du STEP de référence. Suivre les arêtes communes et leur orientation.
- Mesurer la fidélité des limites, contrôler les deux traces paramétriques après transfert et rejouer un contre-exemple par pièce.

## Hors périmètre

La découverte automatique des supports inconnus et leur construction native depuis le STL relèvent des étapes suivantes. Les sections coniques demandent leurs propres témoins. Le rejeu de Flansch est différé jusqu’à la fin des développements de reconnaissance géométrique.

## Dépendances

Les deux références et les mesures initiales d’Auto sont décrites en V-1260. Le générateur utilise FreeCAD.

## Critères d'acceptation

Attribuer chaque triangle au même propriétaire que le mailleur du solide de référence. Vérifier les deux bords communs de chaque pièce. Documenter la portée des bornes continues et des écarts échantillonnés. Une interface déformée ou une attribution incompatible doit faire échouer la preuve. Mesurer le résultat actuel d’Auto aux deux densités pour situer le travail de reconstruction restant.

## Vérifications de l'utilisateur

Valider la qualification de la lèvre et de l’arbre percé, et choisir la suite pour l’intégration de l’arbre dans Auto.

## Résultats

La lèvre torique et son plan partagent deux branches. Le banc vérifie leur couverture et leur écart sur tout le bord aux deux densités. L’arbre percé est également reconnu depuis ses seuls triangles dans le prototype. Les deux STL donnent un solide STEP valide à quatre faces et conservent tous leurs propriétaires source. Une seconde pièce de dimensions différentes passe le même contrôle. Auto produit encore des facettes sur ces témoins. L’intégration dans le bureau demande un domaine natif pour la paroi extérieure percée. Les résultats, contre-exemples et commandes figurent en V-1264.
