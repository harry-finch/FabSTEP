# État — 13 novembre 2026

**V1 mécanique 1.0.0 acceptée ; extensions post-v1 dans le build de développement.** Rust/OCCT, Svelte 5/Tauri, GPL-3.0-or-later. Cible qualifiée : Apple Silicon, macOS 26 ou ultérieur. La v1 correspond au tag `v1.0.0` ; la mise d’équerre est préparée sur `fabboard/mise-d-equerre`, issue de `codex/v1-mechanical`. Les modifications fonctionnelles post-v1 déjà présentes restent non commitées.

## Vérifié et accepté

- La v1 importe STL/3MF Core et exporte les solides STEP des familles mécaniques prouvées. Limites : 32 corps, 64 Mio, 100 000 triangles après placement ; unité STL explicite, refus des contacts et de l’export partiel. Voir [README](README.md) et [jalons livrés](ROADMAP.md).
- Le paquet autonome `artifacts/release/1.0.0/FabSTEP.app` est signé ad hoc et embarque OCCT. L’utilisateur a confirmé le 9 novembre son installation sur un autre Mac et l’utilisation des STEP dans Fusion ([V-1115](docs/VALIDATION.md)). La CI macOS du 7 novembre a réussi. Aucune signature Developer ID ni notarisation acquise.
- L’intégration initiale de l’audit ([R-020](roadmap/items/R-020-int-audit-01-integration-initiale.md)) et PERF1 ([R-051](roadmap/items/R-051-perf1-preuves-source-reutilisees.md)) sont consignés comme validés dans la feuille de route d’origine ; mesures et limites restent dans [VALIDATION](docs/VALIDATION.md) et [INT-AUDIT-01](docs/INT_AUDIT_01.md).

## Développé, qualification encore ouverte

Le build `artifacts/FabSTEP.app` contient les extensions géométriques A/B/C1a, cylindres ouverts décalés et petits raccords ; l’atelier UX4, la sélection source/STEP et la provenance exacte, les indications Cylindre/Cône, les profils de faces et les sections STEP/source DXF. Les courbes DXF prises en charge incluent les ellipses exactes ; la coupe source reste polygonale. Ces fonctions ne font pas partie du paquet autonome 1.0.0.

La dernière validation globale **historique**, V-1177 du 12 novembre, rapporte 391 tests workspace, 32 bureau, 175 UI, 12 Python et 4 SDK, formatage, Clippy strict, Svelte et builds réussis. Ce bilan ne remplace pas les essais natifs et Fusion encore ouverts. La sélection/ouverture manuelle a été confirmée ; le blocage du pilotage automatisé n’établit pas un défaut produit de l’import.

Les lots concernés restent `a-tester` dans [ROADMAP](ROADMAP.md) ; 28 actions détaillent séparément les parcours graphiques et réimports attendus. Leurs réponses doivent être examinées avant d’accepter chaque lot.

## Mise d’équerre en préparation

Ancienne feuille de route conservée dans [l’archive](docs/ROADMAP-avant-fabboard.md) ; suivi détaillé dans `roadmap/`. L’ancien état complet est conservé dans [DEVLOG](DEVLOG.md), entrée du 13 novembre. Skills de la famille installées localement et consignes partagées par `CLAUDE.md`.

Contrôles **relancés le 13 novembre** : `scripts/check.sh` termine avec code 0 — 391 tests workspace, 32 bureau, 175 UI, 12 Python et 4 SDK ; formatage, Clippy strict, Svelte (0 erreur/avertissement), build UI et contrôle bureau réussis. Onze tests Rust opt-in restent ignorés par la suite ordinaire ; avertissement Vite connu sur la taille du bundle. [Journal local](artifacts/mise-d-equerre/check.log). Le format FabBoard, les liens et la conservation exacte des archives ont aussi été contrôlés. La figure Hairline passe son validateur et ses huit vues Chrome, regardées en clair/sombre et en mouvement. Aucun nouvel essai graphique natif ni Fusion n’est revendiqué. Les idées futures et sujets conditionnels restent dans la feuille de route sans nouvelle autorisation de développement.

Validation documentaire reçue : A-030. Réserves de mise d’équerre : [A-029](roadmap/actions/A-029-valider-la-migration-de-la-feuille-de-route-de.md) pour le détail de migration, [A-031](roadmap/actions/A-031-examiner-la-figure-de-la-bride-percee-de-fabstep.md) pour le parcours de la figure et son rendu intégré non confirmé. La validation finale est A-032 dans FabBoard. La branche dédiée n’a pas d’amont, volontairement : aucun push ni fusion.
