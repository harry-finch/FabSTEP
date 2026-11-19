---
id: R-024
titre: "Préserver les caractères composés dans les noms de produits tronqués"
resume: "AUD-011 — Reproduire les cas Unicode utiles ; conserver le nom source et éviter une dépendance disproportionnée pour ce seul besoin"
statut: a-tester
jalon: audit
cree: 2026-11-13
maj: 2026-11-19
livree:
---
## But

Préserver un nom de produit lisible après export.

## Inclus

Limiter le nom STEP à 60 octets UTF-8 en conservant les graphèmes entiers. Garder le nom source dans le rapport et compter les adaptations.

## Hors périmètre

Les identifiants des corps et le format des projets restent ceux du modèle importé.

## Dépendances

Le bureau embarque déjà unicode-segmentation 1.13.3. Le worker et la CLI utilisent désormais cette même version.

## Critères d'acceptation

Accents décomposés, emoji avec modificateur, drapeaux et caractères multioctets aux deux côtés de la limite. Un cube 3MF produit un solide STEP relu et conserve son nom complet dans le rapport.

## Vérifications de l'utilisateur

Ouvrir fixtures/m3_name_grapheme.3mf et vérifier le nom du produit dans la CAO après export.

| Préserver les caractères composés dans les noms de produits tronqués | AUD-011 | P3 | Reproduire les cas Unicode utiles ; conserver le nom source et éviter une dépendance disproportionnée pour ce seul besoin |

Source : [ancienne feuille de route](../../docs/ROADMAP-avant-fabboard.md). Les validations historiques sont conservées ; aucun essai nouveau n’est déduit de la migration.
