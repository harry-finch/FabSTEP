---
id: A-028
type: tester
titre: "Vérifier les coupes polygonales source dans Fusion"
element: R-054
statut: fait
cree: 2026-11-13
par: agent
fait: 2026-11-19
---

Version à essayer : artifacts/FabSTEP.app (build de développement, distinct du paquet autonome 1.0.0). Si le bundle manque, reconstruire avec sh scripts/build-macos-dev.sh ; le lancement de développement est npm --prefix ui run desktop:dev.

Importer les DXF source dans Fusion : dimensions en mm à 1:1, contours polygonaux, trous et composantes préservés. Comparer le repère de l’aperçu ; aucune courbe ni fermeture artificielle attendue.

Répondre avec le résultat, le fichier et les réglages utilisés ; en cas de défaut, décrire les gestes, le résultat attendu et obtenu.

## Réponse

Accepté par l’utilisateur lors de la validation groupée des tests et des jalons du 19 novembre 2026.
