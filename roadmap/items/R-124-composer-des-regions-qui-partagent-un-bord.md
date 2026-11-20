---
id: R-124
titre: "Composer des régions qui partagent un bord vérifié"
resume: "G1 · Décrire les voisinages et les triangles de chaque région afin de combiner plusieurs reconnaissances sur un même corps."
statut: specifiee
jalon: post-v1
cree: 2026-11-20
maj: 2026-11-20
livree:
---
## But

Sur un bloc portant deux opérations distinctes, l’utilisateur doit retrouver les deux dans le STEP, même lorsqu’une autre zone reste facettisée.

## Inclus

- Construire le premier graphe d’adjacence à partir des deux domaines de R-123 et de la composition locale de R-114. Conserver pour chaque région ses triangles et ses boucles orientées.
- Assembler les opérations disjointes et partager une même arête entre voisins. Un conflit de propriété ou un bord invérifiable entraîne un repli local compatible, ou le refus du solide si la fermeture ne peut être prouvée.

## Hors périmètre

Réécriture générale des reconnaisseurs existants et calcul universel des intersections entre surfaces libres.

## Dépendances

R-123 et les témoins de composition de R-114.

## Critères d'acceptation

- Rejouer le bloc perçages–congé de R-114 et une autre combinaison comportant un voisin planaire et un voisin torique. Vérifier les familles, les arêtes communes, la provenance exhaustive et la validité après transfert.
- Inverser l’ordre des propositions sur le même maillage et retrouver les mêmes propriétaires et le même résultat géométrique. Deux régions revendiquant les mêmes triangles ne sont jamais publiées ensemble.
- Sur une pièce comportant une région ambiguë, vérifier le maintien des domaines indépendants si le repli produit un solide entier conforme.

## Vérifications de l'utilisateur

Aucune.
