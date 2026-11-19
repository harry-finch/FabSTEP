---
id: A-091
type: tester
titre: "Contrôler le coin à deux congés adjacents dans FabSTEP et Fusion"
element: R-089
statut: fait
cree: 2026-11-18
par: agent
fait: 2026-11-19
---

Ouvrir fixtures/adjacent_pocket/nominal.stl dans artifacts/FabSTEP.app en millimètres et cliquer sur « Reconstruire le STEP » en mode Automatique. Dans le résultat, cliquer sur chaque arrondi au fond de la poche et vérifier qu’il se sélectionne séparément. Exporter le STEP et le rouvrir dans Fusion : les deux faces de rayon 2 mm doivent partager une seule arête sans facettes au coin. Volume attendu 5 781,55163 mm³.

## Réponse

Accepté par l’utilisateur lors de la validation groupée des tests et des jalons du 19 novembre 2026.
