---
id: R-128
titre: "Proposer des axes et des motifs de parois répétées"
resume: "G6 · Accélérer la recherche des familles et le guidage en utilisant les normales et la répétition des formes."
statut: specifiee
jalon: post-v1
cree: 2026-11-20
maj: 2026-11-20
livree:
---
## But

Une bride ne possède pas toujours un anneau complet pour trouver son axe. Un motif de trous incomplet doit néanmoins aider à sélectionner les parois qui existent réellement.

## Inclus

- Classer les axes candidats d’après les normales pondérées par aire, puis les vérifier sur les triangles.
- Chercher les répétitions linéaires et circulaires. Réutiliser les propositions vérifiées dans le parcours des parois similaires de R-120. Chaque occurrence garde sa preuve et son attribution.

## Hors périmètre

Création d’un trou absent, correction forcée des rayons ou déplacement des centres pour régulariser un motif.

## Dépendances

R-122 et R-120. Les pièces de R-126 et R-127 fournissent des axes de référence supplémentaires.

## Critères d'acceptation

- Retrouver le même axe à deux densités et après rotation d’une pièce sans anneau complet. Un jeu de normales ambigu doit écarter la proposition ou la départager par preuve géométrique.
- Sur une bride à huit positions dont une vide et une de rayon différent, proposer uniquement les parois existantes compatibles. Vérifier également une rangée linéaire irrégulière et le respect du budget de recherche.

## Vérifications de l'utilisateur

Prévisualiser les parois similaires et vérifier que l’emplacement vide et le trou de diamètre différent restent exclus.
