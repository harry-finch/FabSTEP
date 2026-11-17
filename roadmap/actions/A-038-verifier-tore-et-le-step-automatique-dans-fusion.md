---
id: A-038
type: tester
titre: "Vérifier Tore et le STEP automatique dans Fusion"
element: R-057
statut: a-faire
cree: 2026-11-16
par: agent
fait:
---

Dans artifacts/FabSTEP.app reconstruit le 16 novembre, ouvrir fixtures/m5_toroidal_fillet.stl. Auto reconnaît le raccord. Double-cliquer sur le raccord : le groupe doit inclure sa portée cylindrique nécessaire, sans ses plans voisins. Choisir Tore puis Essayer ; retirer/recalculer l’indication. Vérifier la lisibilité des commandes et des limites. Exporter puis ouvrir le STEP dans Fusion et vérifier solide, surfaces et dimensions ; faire également le contrôle sur body 26mm.stl. Signaler tout écart. Les boîtiers arrondis spécialisés restent dans Avancé > Surfaces arrondies ; les raccords toriques libres ne sont pas pris en charge.
