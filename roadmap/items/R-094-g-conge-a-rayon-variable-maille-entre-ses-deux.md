---
id: R-094
titre: "G — congé à rayon variable maillé entre ses deux extrémités"
resume: "Retrouver le congé linéaire sur une paroi dont les sommets appartiennent tous aux deux bords elliptiques."
statut: a-tester
jalon: post-v1
cree: 2026-11-18
maj: 2026-11-18
livree:
---
## But

Reconstruire un congé linéaire entre deux plans orthogonaux lorsque les triangles relient directement les deux arrêts. Certaines tessellations représentent une génératrice droite par une seule arête.

## Inclus

Deux coupes elliptiques prouvées, rayon croissant ou décroissant, témoins intérieurs pris sur une génératrice du maillage, propriété de toutes les facettes, diagnostic et contrôle natif du solide.

## Hors périmètre

Les lois de rayon non linéaires, les congés qui se rejoignent et les arrêts singuliers relèvent de R-037.

## Dépendances

R-082 fournit la surface conique inclinée, les limites elliptiques et la borne continue.

## Critères d'acceptation

Six plans et un cône après réimport, deux ellipses partagées, volume théorique et provenance contrôlés. Refuser l’empilement de quarts de cercle et respecter le budget d’écart.

## Vérifications de l'utilisateur

- Reconstruire le témoin à rayon croissant et inspecter le cône et ses deux bords dans Fusion.
