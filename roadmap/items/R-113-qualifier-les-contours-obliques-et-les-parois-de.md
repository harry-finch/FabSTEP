---
id: R-113
titre: "Qualifier les contours obliques et les parois de révolution interrompues"
resume: "Comparer une découpe plane de tore et une paroi cylindrique ou conique interrompue sur des solides indépendants. Prouver la courbe commune, la provenance et la fidélité avant toute extension du modèle."
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

Relire le bilan des deux formes et des limites de la reconstruction dans FabBoard.
