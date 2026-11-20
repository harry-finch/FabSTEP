---
id: R-022
titre: "Réduire le coût de démarrage du frontend"
resume: "AUD-009 — Mesurer le démarrage réel de la WebView ; retenir un chargement différé ou un découpage uniquement si le gain est démontré"
statut: livree
jalon: audit
cree: 2026-11-13
maj: 2026-11-20
livree: 2026-11-20
---
## But

Rendre le bouton d’ouverture disponible plus tôt.

## Inclus

Charger le composant de vue 3D à la première ouverture. Tester le premier dessin du maillage en WebView macOS.

## Hors périmètre

Le moteur de conversion et le protocole d’import sont indépendants de ce jalon.

## Dépendances

Vite, import asynchrone natif et WKWebView.

## Critères d'acceptation

Comparer deux builds de production dans des WebViews fraîches, mesurer le délai d’ouverture et le coût reporté au premier modèle. Vérifier contours, sélection et recadrage après ouverture.

## Vérifications de l'utilisateur

Ouvrir un STL dans le build de développement, tourner la pièce puis ouvrir un deuxième fichier.

| Réduire le coût de démarrage du frontend | AUD-009 | P3 | Mesurer le démarrage réel de la WebView ; retenir un chargement différé ou un découpage uniquement si le gain est démontré |

Source : [ancienne feuille de route](../../docs/ROADMAP-avant-fabboard.md). Les validations historiques sont conservées ; aucun essai nouveau n’est déduit de la migration.
