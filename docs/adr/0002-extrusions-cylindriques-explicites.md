# ADR-0002 : Reconstruire des extrusions cylindriques en mode explicite

Statut : accepté
Date : 2026-11-06

## Contexte

Un polygone régulier peut représenter une pièce réellement polygonale ou un cercle tessellisé. Le STL ne permet pas de retrouver cette intention. Le premier jalon conserve donc les facettes en mode Plans. La suite doit produire des surfaces cylindriques analytiques sans inventer une fermeture ou ignorer une partie de la paroi.

## Décision

Ajouter un mode **Cylindres** explicite, strict et réversible. Il reconnaît seulement une extrusion droite complète à section constante, avec au moins un cercle : cylindre, tube ou profil polygonal percé de trous circulaires traversants. Un échec est une erreur codée ; l'utilisateur peut choisir Plans, sans repli silencieux.

Le core ajuste les cercles sans exclure de points, exige des bouchons correspondants et vérifie tous les triangles latéraux, subdivisions axiales comprises. La provenance des triangles distingue les bouchons des parois : la seule proximité en altitude d'un bouchon ne suffit pas. La recherche des bouchons utilise le maximum des tolérances de planéité et d'ajustement ; les écarts effectivement rencontrés entrent dans le rapport. Le maillage source reste conservé.

La représentation `AnalyticExtrusion` fournit un repère orthonormal, une hauteur et des profils 2D polygonaux/circulaires. L'adaptateur OCCT construit les faces et leur extrusion. L'orchestrateur valide d'abord le solide des triangles source exacts, puis le résultat analytique et son réimport STEP. Le mode Plans continue d'utiliser les régions fusionnées.

Le budget exposé est une somme conservatrice des résidus vérifiés de soudage, d'extrusion et d'ajustement radial, plus la flèche des cordes. Il ne constitue pas une mesure bidirectionnelle globale de Hausdorff. Les mesures de volume et de dimensions restent des contrôles supplémentaires, pas une preuve suffisante de fidélité.

## Alternatives considérées

- Reconnaissance automatique permanente : rejetée car elle change implicitement l'intention des polygones.
- Reconstruction locale de cylindres sur toute pièce mixte : reportée ; elle impose une gestion fiable des intersections, raccords et surfaces voisines.
- Tolérance native relâchée pour accepter les STL binaires : rejetée ; valider les triangles source évite de forcer la planéité d'un contour fusionné.

## Conséquences

L'API de reconstruction est additive et le mode par défaut reste Plans. L'interface distingue les faces planes et cylindriques et expose les deux tolérances du mode. Deux solides sont construits/contrôlés en mode Cylindres : le coût de validation augmente sur les gros maillages.

Trous borgnes, bossages, extrusions obliques, cônes, congés et profils à sections variables restent hors de ce jalon. L'import 3MF reste indépendant.

## Validation

Corpus analytique indépendant : cylindre simple, subdivision axiale, rotation/translation, arrondi binaire f32, tube et plaque percée. Cas négatifs : cône, constriction intermédiaire, excroissance près d'un bouchon, polygone insuffisamment échantillonné et budget insuffisant. Réimport indépendant FreeCAD : solide fermé valide, types des surfaces, rayons et volumes. Les résultats exécutés sont consignés dans `../VALIDATION.md`.
