---
id: A-009
type: tester
titre: "Essayer les chaînes coniques dans le bureau"
element: R-031
statut: fait
cree: 2026-11-13
par: agent
fait: 2026-11-19
---

Version à essayer : artifacts/FabSTEP.app (build de développement, distinct du paquet autonome 1.0.0). Si le bundle manque, reconstruire avec sh scripts/build-macos-dev.sh ; le lancement de développement est npm --prefix ui run desktop:dev.

Utiliser fixtures/post_v1_chains/ en Primitives et Surfaces ; vérifier les chaînes alternées avec portées, puis l’export et le refus des cas non admis (V-1125 à V-1127).

Répondre avec le résultat, le fichier et les réglages utilisés ; en cas de défaut, décrire les gestes, le résultat attendu et obtenu.

## Réponse

Accepté par l’utilisateur lors de la validation groupée des tests et des jalons du 19 novembre 2026.
