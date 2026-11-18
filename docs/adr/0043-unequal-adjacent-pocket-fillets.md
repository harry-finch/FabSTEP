# ADR-0043 — Jonction de deux congés de rayons différents

**Décision acceptée le 18 novembre 2026 dans A-092.** R-090 concerne deux axes horizontaux perpendiculaires dans une poche à fond plan. Les rayons sont constants, distincts et mesurables sur le maillage. Chaque congé a une butée droite à son extrémité libre.

## Géométrie

Soient `R > r > 0` les rayons. Placer l’origine au coin virtuel formé par le fond et les deux parois. Pour une hauteur `h` comprise entre 0 et `r`, la branche commune a pour coordonnées :

```text
x = R − √(h(2R − h))
y = r − √(h(2r − h))
z = h
```

La courbe part du fond. À la hauteur `r`, elle atteint la paroi verticale du petit congé. Le grand cylindre se prolonge ensuite jusqu’à la hauteur `R` et partage avec cette paroi un arc de cercle de rayon `R` et d’angle `asin(1 − r/R)`. Le petit cylindre a quatre arêtes, le grand en a cinq. Le cas de même rayon reste décrit par ADR-0042.

## Contrat retenu

Le domaine interne optionnel `RoundedModel.unequal_pocket` décrit le coin virtuel au niveau du fond, les normales des deux parois et du fond, les dimensions mesurées de la poche et du bloc et les deux rayons. Ce repère admet les deux orientations, ce qui couvre les pièces en miroir. Ces mesures fixent les deux cercles de butée, les extrémités de la jonction et le troisième arc circulaire. Les supports mesurés déterminent la courbe spatiale et ses deux représentations paramétriques. Le STEP conserve deux faces cylindriques identifiées séparément, une seule arête commune et l’arc supplémentaire partagé avec une face plane. Le profil DXF de cette dernière doit conserver cet arc. Les anciens modèles gardent leur représentation.

La jonction admet une Bézier rationnelle de degré quatre, à cinq pôles. Pour deux rayons strictement distincts, elle est non plane et l’ellipse de R-089 introduirait un écart. Le laboratoire fournit les coefficients homogènes, la preuve algébrique, les tracés sur les deux cylindres et les mesures de transfert. [Validation V-1217](../VALIDATION.md#v-1217--qualification-du-coin-à-deux-rayons-distincts-r-090)

L’autre option est de conserver les faces planes déjà exportées. Elle conserve un solide éditable mais laisse deux parois courbes découpées en facettes.

## Conditions d’intégration

- Mesurer indépendamment les deux rayons, l’orthogonalité, les tangences et les deux terminaisons. Vérifier la couverture intégrale et orientée des bandes, les incidences et l’arc supplémentaire entre la paroi plane et le grand cylindre. La distinction des rayons doit être résolue aux tolérances demandées.
- Borner l’écart sur chaque triangle et la courbe entière. Comparer le volume à une référence indépendante. Une déformation locale ne doit pas devenir un coin analytique erroné.
- Réimporter le STEP et contrôler la fermeture, la géométrie de la quartique, son partage par les deux cylindres et les deux tracés paramétriques. Vérifier les deux sens de parcours, la tessellation et l’attribution des faces, y compris pour plusieurs corps. Le lecteur distingue ce domaine de celui des perçages sécants de R-083.
- Valider sur les deux maillages FreeCAD et les témoins publics. Les sondes de laboratoire partent d’une géométrie connue. Les essais à partir des triangles et le contrôle indépendant du STEP issu de la CLI figurent dans [V-1218](../VALIDATION.md#v-1218--conversion-stl-des-deux-congés-adjacents-de-rayons-différents-r-090).
