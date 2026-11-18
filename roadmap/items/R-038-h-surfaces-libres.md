---
id: R-038
titre: "H — surfaces libres"
resume: "Patch bicubique de type hauteur, quatre limites source sur un solide fermé"
statut: a-tester
jalon: post-v1
cree: 2026-11-13
maj: 2026-11-18
livree:
---
## But

Exporter en STEP un relief bicubique mesuré sur la face supérieure d’un socle rectangulaire.

## Inclus

Le mode avancé « Relief sur base rectangulaire » ajuste quatre hauteurs intérieures. La base, les quatre côtés et la rive horizontale sont présents dans le STL. Un certificat borne l’écart sur chaque triangle du toit. Le volume, les six faces, leur provenance et le STEP réimporté sont contrôlés. La reconnaissance accepte au plus 32 000 triangles par corps.

## Hors périmètre

Surplombs, pièces obliques relativement à XY, rives courbes ou inclinées, raccords entre plusieurs surfaces libres et approximation en mode Auto.

## Dépendances

Contrat retenu dans ADR-0044. Mesures indépendantes et contrôles de refus dans V-1228.

## Critères d'acceptation

- Les deux reliefs indépendants donnent cinq plans et un patch bicubique à deux résolutions.
- Les STEP restent fermés, de volume conforme et d’écart inférieur au budget.
- Les sources ouvertes, les ajustements indéterminés ou hors budget sont refusés.

## Vérifications de l'utilisateur

- Reconstruire les pièces de fixtures/height_patch avec le réglage avancé, puis vérifier le relief et le socle dans Fusion.
