# État — 7 novembre 2026

**1.0.0 — v1 mécanique locale.** FabApps, Rust/OCCT, Svelte/Tauri, GPL-3.0-or-later. Cible : Apple Silicon, macOS 26 ou ultérieur. Référence Git : tag local `v1.0.0`, branche `codex/v1-mechanical`, après la candidate `030fd56`.

- STL et 3MF Core, unités et placements conservés ; 32 corps, 64 Mio et 100 000 triangles maximum. Aucun export partiel ni corps en contact.
- Plans, cylindres complets et ouverts, pièces mixtes et borgnes plans, cônes et doubles chanfreins résolus, calottes et sphères entières, tores entiers, congés toriques quart cylindre–plan et boîtiers convexes entièrement arrondis à rayon uniforme.
- Les quatre positifs et deux négatifs du [corpus indépendant](fixtures/complex_v1/README.md) passent. Six STEP issus des deux pièces utilisateur ont été réimportés dans FreeCAD. [Validation](docs/VALIDATION.md)
- 204 tests workspace, cinq bureau et sept UI réussis en 1.0.0 ; formatage, Clippy strict, Svelte et build réussis. La revue indépendante confirme l’absence de changement fonctionnel depuis RC1.
- Transfert natif STL/3MF, aperçu et export de deux cubes observés automatiquement. Aperçu et export du boîtier à 26 faces confirmés par l’utilisateur sur RC1 ; cet export a passé son réimport FreeCAD indépendant, avec 48 raccords tangents.
- Paquet autonome `artifacts/release/1.0.0/FabSTEP.app`, ZIP, notices et 29 bibliothèques, signature ad hoc. Conversions de contrôle exécutées avec lectures Homebrew bloquées. [Livraison](docs/RELEASE.md)

**Suite** — essais sur un autre Mac et dans Fusion, puis distribution signée/notarisée et Windows/Linux. Congés partiels ou à rayon variable, boîtiers creux/percés avec arrondis, intersections de perçages, surfaces libres et 3MF Production multi-modèles restent hors du périmètre reconnu. Aucun distant Git ni publication ; la v1 locale ne prétend pas couvrir toutes les géométries mécaniques.
