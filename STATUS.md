# État — 7 novembre 2026

**1.0.0-rc.1 — candidate mécanique ; qualification native finale en cours.** FabApps, Rust/OCCT, Svelte/Tauri, GPL-3.0-or-later. Cible locale : Apple Silicon, macOS 26 ou ultérieur.

- STL et 3MF Core, unités et placements conservés, 32 corps et 100 000 triangles maximum ; pas d’export partiel ni de corps en contact.
- Plans, cylindres complets et ouverts, pièces mixtes et borgnes plans, cônes et doubles chanfreins résolus, calottes et sphères entières, tores entiers, congés toriques quart cylindre–plan, boîtiers convexes entièrement arrondis à rayon uniforme.
- Les quatre positifs et les deux négatifs du [corpus mécanique indépendant](fixtures/complex_v1/README.md) passent. Les deux pièces utilisateur conservent leurs surfaces et mesures dans les deux formats ; six STEP privés ont été réimportés dans FreeCAD. [Validation](docs/VALIDATION.md)
- Audit de distance optimisé sans modifier les échantillons, les distances ni son budget. Le carter de 41 290 triangles passe désormais sous ce budget.
- Réglages et erreurs structurés, aide FR/EN, transport binaire de la source avec cache natif par identité. 204 tests workspace, cinq bureau et sept UI passent ; formatage, Clippy strict, Svelte et build réussis. Transfert natif STL/3MF et export de deux cubes observés. Le pilotage perd l’accès à la fenêtre après import par dialogue ; vérification manuelle du boîtier demandée.
- Paquet autonome avec notices, signature ad hoc et contrôles des bibliothèques. La candidate intègre les bibliothèques nécessaires aux congés. Pas de notarisation ou publication. [Livraison](docs/RELEASE.md)

**Suite active** — confirmer le parcours du boîtier dans le paquet candidat. La vérification manuelle a été demandée après pertes répétées d’accès de l’outil ; aucune cause applicative n’est établie. Le premier historique local est préparé sur `codex/v1-mechanical`. La v1 n’est pas encore déclarée achevée. Fusion, autres Mac, Windows/Linux, intersections de perçages, congés généraux à rayon variable, surfaces libres et 3MF Production multi-modèles restent non qualifiés. Aucun distant Git configuré.
