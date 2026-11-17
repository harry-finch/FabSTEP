# Chaînes coniques de même signe avec portées — C1b1

Corpus synthétique indépendant, en **millimètres**. Les profils nominaux, volumes, nombres de faces, bornes et empreintes sont définis dans `manifest.json` avant conversion. Régénération : `python3 fixtures/generate_same_sign_chains.py`.

Pour un essai dans FabSTEP, ouvrir `two_increasing.stl` en Automatique : deux cônes et deux faces cylindriques (portée et alésage), avec deux faces planes. `three_decreasing.stl` contient trois cônes descendants ; `mixed_signs.stl` mélange les sens. `two_rotated.stl`, `binary.stl` et `instances.3mf` vérifient respectivement transformation, quantification STL f32 et deux instances placées dont une en miroir. Le manifeste donne les mesures attendues par fichier.

`no_land.stl`, `curved_meridian.stl` et `late_ambiguous_land.stl` sont des cas de refus géométrique : absence de portée, profil courbe, ou dernière portée insuffisamment résolue. Les modes explicites ne doivent pas forcer des cônes ; Auto conserve les régions non prouvées en facettes après validation du solide entier.

Les tests automatisés sont dans `crates/fabstep-core/tests/conical_chains.rs` et `crates/fabstep-app/tests/same_sign_conical_chains.rs`. Leur passage ne remplace pas le contrôle visuel et le réimport dans Fusion. Voir V-1184 dans `docs/VALIDATION.md` pour les résultats exécutés.
