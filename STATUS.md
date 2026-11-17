# État — 17 novembre 2026

**V1 mécanique 1.0.0 acceptée ; extensions post-v1 dans le build de développement.** Rust/OCCT, Svelte 5/Tauri, GPL-3.0-or-later. Cible qualifiée : Apple Silicon, macOS 26 ou ultérieur. La v1 correspond au tag `v1.0.0` ; la mise d’équerre est préparée sur `fabboard/mise-d-equerre`, issue de `codex/v1-mechanical`. Les modifications fonctionnelles post-v1 déjà présentes restent non commitées.

## Vérifié et accepté

- La v1 importe STL/3MF Core et exporte les solides STEP des familles mécaniques prouvées. Limites : 32 corps, 64 Mio, 100 000 triangles après placement ; unité STL explicite, refus des contacts et de l’export partiel. Voir [README](README.md) et [jalons livrés](ROADMAP.md).
- Le paquet autonome `artifacts/release/1.0.0/FabSTEP.app` est signé ad hoc et embarque OCCT. L’utilisateur a confirmé le 9 novembre son installation sur un autre Mac et l’utilisation des STEP dans Fusion ([V-1115](docs/VALIDATION.md)). La CI macOS du 7 novembre a réussi. Aucune signature Developer ID ni notarisation acquise.
- L’intégration initiale de l’audit ([R-020](roadmap/items/R-020-int-audit-01-integration-initiale.md)) et PERF1 ([R-051](roadmap/items/R-051-perf1-preuves-source-reutilisees.md)) sont consignés comme validés dans la feuille de route d’origine ; mesures et limites restent dans [VALIDATION](docs/VALIDATION.md) et [INT-AUDIT-01](docs/INT_AUDIT_01.md).

## Développé, qualification encore ouverte

Le build `artifacts/FabSTEP.app` contient les extensions géométriques A/B/C1a, cylindres ouverts décalés et petits raccords ; l’atelier UX4, la sélection source/STEP et la provenance exacte, les indications Plan/Cylindre/Cône, les profils de faces et les sections STEP/source DXF. Les courbes DXF prises en charge incluent les ellipses exactes ; la coupe source reste polygonale. Ces fonctions ne font pas partie du paquet autonome 1.0.0.

La dernière validation globale **historique**, V-1177 du 12 novembre, rapporte 391 tests workspace, 32 bureau, 175 UI, 12 Python et 4 SDK, formatage, Clippy strict, Svelte et builds réussis. Ce bilan ne remplace pas les essais natifs et Fusion encore ouverts. La sélection/ouverture manuelle a été confirmée ; le blocage du pilotage automatisé n’établit pas un défaut produit de l’import.

Les lots concernés restent `a-tester` dans [ROADMAP](ROADMAP.md) ; 28 actions détaillent séparément les parcours graphiques et réimports attendus. Leurs réponses doivent être examinées avant d’accepter chaque lot.

## Reprise UX4 du 15 novembre

Le retour utilisateur sur l’inspecteur trop chargé et le guidage introuvable est traité dans [R-042](roadmap/items/R-042-ux4-simplifier-latelier.md) : barre contextuelle Cylindre/Cône/Essayer près du modèle, préparation explicite depuis un résultat automatique, détails techniques repliés et ouverture du panneau à une nouvelle sélection. **181 tests UI**, Svelte, build macOS et contrôle bureau réussis ; revue sans défaut bloquant. Rendu interactif restant à qualifier : serveur local refusé et navigateur de pilotage indisponible. Preuves et limites : V-1178 dans [VALIDATION](docs/VALIDATION.md).

## Plan guidé — 15 novembre

[R-056](roadmap/items/R-056-i2d-fusion-plane-guidee.md) ajoute Plan à la correction contextuelle : fusion de faces polygonales contiguës, sans déplacement des coordonnées source, avec refus des contours courbes. Un faible relief peut être simplifié dans la tolérance ; son résidu reste signalé. Revue indépendante acceptée et contrôles globaux réussis : **400 tests workspace, 32 bureau, 184 UI, 12 Python et 4 SDK**. Qualification graphique/Fusion encore ouverte ; preuves V-1180 dans [VALIDATION](docs/VALIDATION.md). Le guidage Tore est désormais intégré à UX5 ci-dessous.

## UX5 — automatique et sélection des parois

[R-057](roadmap/items/R-057-ux5-reconstruction-automatique-et-selection-des.md) ajoute Auto par défaut, sélection géométrique par double-clic et correction locale sans perte des régions indépendantes, avec Tore et sa portée cylindrique. Stratégies historiques dans Avancé, paramètres conservés. **415 tests workspace, 32 bureau, 197 UI, 12 Python et 4 SDK**, Clippy, formatage et builds passent ; revue indépendante acceptée. Essai final body 26mm validé avec 6 cylindres et 3 cônes, tolérances initiales ; paquet `artifacts/FabSTEP.app` reconstruit et empreinte vérifiée. V-1181 dans [VALIDATION](docs/VALIDATION.md) détaille preuves et limites : boîtier arrondi spécialisé en mode avancé, contours couplés ambigus conservateurs, graphique/Fusion non qualifiés et sock_clip absent de son ancien chemin.

## Ajustement de la sélection

Le retour utilisateur sur le double-clic est traité par un balayage manuel indépendant de la reconnaissance : clic gauche maintenu pour ajouter les faces visibles, Maj-clic pour retirer une face, bouton droit pour tourner. Un double-clic sans extension affiche désormais une aide explicite. **204 tests UI**, Svelte et build passent ; qualification graphique impossible ici, restant à faire sur le bureau. Preuves V-1182 dans [VALIDATION](docs/VALIDATION.md), suivi R-057.

## Raccords ouverts cylindre–chanfrein

[R-058](roadmap/items/R-058-raccords-ouverts-cylindre-chanfrein.md), approuvé par l’utilisateur, traite le refus sur la paroi cylindrique de body 26mm en reconstruisant aussi le chanfrein conique partiel et la bande inférieure. Auto et correction guidée sont validés nativement : 1 solide, 14 faces cylindriques et 7 coniques, 144 triangles sélectionnés étendus aux 432 triangles du raccord. Tolérances initiales conservées. Revue indépendante acceptée ; **423 tests workspace, 32 bureau, 208 UI, 12 Python et 4 SDK** réussis, compléments ciblés finaux et analyse statique passent. L’interruption initiale des doctests par conflit d’artefacts a été résolue par un rejeu séquentiel. Build `artifacts/FabSTEP.app` reconstruit, empreinte vérifiée. V-1183 détaille les preuves et la portée demi-circulaire ; correctif accepté par l’utilisateur le 17 novembre (A-040), réimport Fusion encore non confirmé (A-041). Le refus guidé affiche désormais « Correction non appliquée » et le recalcul est directement accessible.

## Chaînes coniques et interface — 17 novembre

R-032 ajoute les chaînes de deux à huit cônes de signes libres avec portées cylindriques résolues et témoins source intérieurs ; revue indépendante acceptée. Les cônes sans portée restent dans R-059. R-042 simplifie ensuite l’inspecteur : détails techniques repliés, Essayer primaire, actions utiles conservées et répétitions retirées. **433 tests workspace, 32 bureau, 208 UI, 12 Python et 4 SDK** réussis ; formatage, Clippy strict, Svelte et builds passent. `artifacts/FabSTEP.app` reconstruit, empreinte vérifiée. Preuves V-1184 ; essais graphiques et Fusion non exécutés, suivis par A-042 à A-044. Lots à tester, sans nouvelle publication distante.

## Mise d’équerre acceptée

Ancienne feuille de route conservée dans [l’archive](docs/ROADMAP-avant-fabboard.md) ; suivi détaillé dans `roadmap/`. L’ancien état complet est conservé dans [DEVLOG](DEVLOG.md), entrée du 13 novembre. Skills de la famille installées localement et consignes partagées par `CLAUDE.md`.

Contrôles **relancés le 13 novembre** : `scripts/check.sh` termine avec code 0 — 391 tests workspace, 32 bureau, 175 UI, 12 Python et 4 SDK ; formatage, Clippy strict, Svelte (0 erreur/avertissement), build UI et contrôle bureau réussis. Onze tests Rust opt-in restent ignorés par la suite ordinaire ; avertissement Vite connu sur la taille du bundle. [Journal local](artifacts/mise-d-equerre/check.log). Le format FabBoard, les liens et la conservation exacte des archives ont aussi été contrôlés. La figure Hairline passe son validateur et ses huit vues Chrome, regardées en clair/sombre et en mouvement. Aucun nouvel essai graphique natif ni Fusion n’est revendiqué. Les idées futures et sujets conditionnels restent dans la feuille de route sans nouvelle autorisation de développement.

Validation documentaire reçue : A-030. Réserves de mise d’équerre : [A-029](roadmap/actions/A-029-valider-la-migration-de-la-feuille-de-route-de.md) pour le détail de migration, [A-031](roadmap/actions/A-031-examiner-la-figure-de-la-bride-percee-de-fabstep.md) pour le parcours de la figure et son rendu intégré non confirmé. La validation d’organisation A-032 a été reçue le 16 novembre ; elle ne vaut pas acceptation des extensions fonctionnelles. La branche dédiée n’a pas d’amont, volontairement : aucun push ni fusion.
