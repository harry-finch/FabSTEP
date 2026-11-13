---
id: A-005
type: tester
titre: "Essayer les cylindres ouverts décalés dans le bureau"
element: R-029
statut: a-faire
cree: 2026-11-13
par: agent
fait:
---

Version à essayer : artifacts/FabSTEP.app (build de développement, distinct du paquet autonome 1.0.0). Si le bundle manque, reconstruire avec sh scripts/build-macos-dev.sh ; le lancement de développement est npm --prefix ui run desktop:dev.

Utiliser fixtures/partial_staggered/ et, si disponible, la pièce privée déjà testée. Vérifier la conversion dans les trois modes analytiques, l’aperçu et l’export. Sur la pièce privée, la tolérance radiale historique explicite est 0,00002 mm (V-1149 à V-1152).

Répondre avec le résultat, le fichier et les réglages utilisés ; en cas de défaut, décrire les gestes, le résultat attendu et obtenu.
