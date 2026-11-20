---
id: R-127
titre: "Reconstruire une extrusion droite à profil libre"
resume: "G3 · Reconnaître un contour extrudé composé de segments, d’arcs et de portions cubiques."
statut: specifiee
jalon: post-v1
cree: 2026-11-20
maj: 2026-11-20
livree:
---
## But

Une came ou une platine découpée doit retrouver ses flancs continus et ses trous lorsque la même section traverse toute son épaisseur.

## Inclus

- Identifier une direction candidate. Vérifier la correspondance de deux sections et la continuité des flancs entre elles.
- Ajuster chaque boucle 2D, y compris les trous, avec des segments, arcs ou cubiques bornées. Préserver les angles vifs et vérifier les raccords lisses sur les données.

## Hors périmètre

La dépouille constante demande une qualification distincte après le cas droit. Les profils torsadés ou variables restent hors de ce domaine.

## Dépendances

R-122 et extrusions existantes R-003. Indépendant de la méridienne R-126.

## Critères d'acceptation

- Convertir une came et un profilé percé à deux densités. Vérifier après réimport la section, les trous, l’épaisseur, le volume, la provenance et les deux directions de fidélité.
- Une pièce aux extrémités identiques mais cintrée entre les deux, une pièce vrillée et un profil à faible dépouille doivent écarter la proposition d’extrusion droite.

## Vérifications de l'utilisateur

Contrôler la section de la came et la continuité de son flanc dans Fusion.
