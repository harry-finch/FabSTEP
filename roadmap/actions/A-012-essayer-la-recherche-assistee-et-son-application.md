---
id: A-012
type: tester
titre: "Essayer la recherche assistée et son application explicite"
element: R-040
statut: fait
cree: 2026-11-13
par: agent
fait: 2026-11-19
---

Version à essayer : artifacts/FabSTEP.app (build de développement, distinct du paquet autonome 1.0.0). Si le bundle manque, reconstruire avec sh scripts/build-macos-dev.sh ; le lancement de développement est npm --prefix ui run desktop:dev.

Sur un fichier reconnu en mode analytique, lancer Rechercher un réglage. Vérifier que les essais ne remplacent pas l’export courant, puis Appliquer et recalculer. Changer source/options pendant une recherche et vérifier qu’une proposition périmée ne s’applique pas. Voir ADR-0023.

Répondre avec le résultat, le fichier et les réglages utilisés ; en cas de défaut, décrire les gestes, le résultat attendu et obtenu.

## Réponse

Accepté par l’utilisateur lors de la validation groupée des tests et des jalons du 19 novembre 2026.
