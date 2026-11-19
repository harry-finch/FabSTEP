---
id: R-025
titre: "Découper les gros modules et mutualiser les helpers au besoin"
resume: "AUD-021 — Intervenir par module lors d’un changement fonctionnel justifié, avec tests ; aucune refonte globale fondée uniquement sur le nombre de lignes"
statut: a-tester
jalon: audit
cree: 2026-11-13
maj: 2026-11-19
livree:
---
## But

Isoler une responsabilité lors d’un changement fonctionnel.

## Inclus

Extraire la gestion de l’historique borné des indications à l’occasion de R-049 et le traitement des noms à l’occasion de R-024.

## Hors périmètre

Les autres modules restent candidats à une extraction lorsqu’un changement fonctionnel la justifie.

## Dépendances

R-049 et R-024.

## Critères d'acceptation

Une seule politique de budget de l’historique, tests du dépassement, des retours arrière et des graphèmes dans les modules concernés.

## Vérifications de l'utilisateur

Le parcours de recette de R-049 couvre ce changement interne.

| Découper les gros modules et mutualiser les helpers au besoin | AUD-021 | P3 | Intervenir par module lors d’un changement fonctionnel justifié, avec tests ; aucune refonte globale fondée uniquement sur le nombre de lignes |

Source : [ancienne feuille de route](../../docs/ROADMAP-avant-fabboard.md). Les validations historiques sont conservées ; aucun essai nouveau n’est déduit de la migration.
