---
id: R-125
titre: "Étendre l’atlas aux parois analytiques interrompues"
resume: "G2 et suite de F1 · Partager des contours vérifiés entre tores, plans et parois cylindriques, puis étendre famille par famille."
statut: livree
jalon: post-v1
cree: 2026-11-20
maj: 2026-11-20
livree: 2026-11-20
---
## But

Une lumière ou un bossage interrompt souvent une paroi régulière. L’atelier doit permettre de sélectionner la paroi reconnue en CAO tout en conservant la découpe réelle.

## Inclus

- Commencer par un cylindre voisin d’un congé torique et par la lèvre torique à coupe oblique de R-113. Vérifier les deux branches de la coupe.
- Étendre ensuite aux cônes, puis aux sphères interrompues sur témoins distincts. Pour chaque famille, vérifier les bords partagés, les coutures périodiques et le voisin qui ferme la découpe.

## Hors périmètre

Surfaces libres et reconstruction d’une découpe dont les deux côtés ne permettent pas de prouver le contour.

## Dépendances

R-123 et R-124. Réutiliser les domaines bornés de R-113 et R-117, notamment leurs contre-exemples.

## Critères d'acceptation

- Pour chaque extension, convertir en Auto deux pièces de dimensions différentes à deux densités. Vérifier après relecture la famille du support, la découpe, la propriété des triangles et les deux écarts aux tolérances existantes.
- Une branche manquante, une couture mal appariée et un voisin presque tangent doivent laisser la représentation sûre ou entraîner le refus du solide. Comptabiliser séparément les gains de faces et de colliers par famille.

## Vérifications de l'utilisateur

Dans Fusion, sélectionner la paroi interrompue et inspecter les deux bords de la lèvre sur le témoin exporté.

## Qualification

Les quatre lèvres toriques donnent cinq faces et les huit bandes coniques ou sphériques quatre faces. Les contrôles natifs des deux branches et la recette indépendante sont consignés en V-1282 et V-1284.
