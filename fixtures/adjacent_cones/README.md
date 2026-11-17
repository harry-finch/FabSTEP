# Deux cônes directement raccordés — R-060

Corpus indépendant en millimètres, généré par `python3 fixtures/generate_adjacent_cones.py` depuis des profils nominaux à anneaux intérieurs. `manifest.json` fixe les volumes, surfaces, dimensions et empreintes avant reconnaissance.

`increasing.stl` : deux cônes montants adjacents, alésage R5, deux plans, cinq faces au total ; volume 1440π mm³, dimensions 32 × 32 × 9 mm. `decreasing.stl`, `opposed.stl` et `rotated.stl` vérifient les autres sens et le placement. Le STL `../same_sign_chains/no_land.stl` est également un positif ; les sources historiques restent inchangées.

Les profils sans témoins, les courbes, les ruptures ambiguës et les chaînes de trois cônes restent hors de cette première tranche. Cas synthétiques adversaires et paroi concave dans `crates/fabstep-core/tests/adjacent_cones.rs`. Réimport et provenance dans le test applicatif homonyme. Les tests natifs ne remplacent pas le contrôle utilisateur dans Fusion ; voir V-1186 dans `docs/VALIDATION.md`.
