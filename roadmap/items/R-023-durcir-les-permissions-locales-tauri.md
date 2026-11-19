---
id: R-023
titre: "Durcir les permissions locales Tauri"
resume: "AUD-014 — Définir le manifeste applicatif et tester les commandes autorisées/refusées, fenêtres et origines ; ne pas ajouter un fichier de capacités supposé suffisant"
statut: a-tester
jalon: audit
cree: 2026-11-13
maj: 2026-11-19
livree:
---
## But

Réserver les commandes natives à la fenêtre d’atelier locale.

## Inclus

Déclarer les commandes explicitement dans le manifeste applicatif. Vérifier la résolution des permissions pour la fenêtre principale, une seconde fenêtre et une origine distante.

## Hors périmètre

Les permissions système macOS et la distribution sont des jalons distincts.

## Dépendances

Registre Tauri du bureau, y compris les trois commandes de R-049.

## Critères d'acceptation

Les commandes autorisées correspondent exactement au registre. Le parcours local d’import, reconstruction, inspection et export fonctionne.

## Vérifications de l'utilisateur

Vérifier l’export depuis la boîte de sauvegarde native de la fenêtre principale.

| Durcir les permissions locales Tauri | AUD-014 | P3 | Définir le manifeste applicatif et tester les commandes autorisées/refusées, fenêtres et origines ; ne pas ajouter un fichier de capacités supposé suffisant |

Source : [ancienne feuille de route](../../docs/ROADMAP-avant-fabboard.md). Les validations historiques sont conservées ; aucun essai nouveau n’est déduit de la migration.
