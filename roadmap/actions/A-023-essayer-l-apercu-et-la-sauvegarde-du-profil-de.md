---
id: A-023
type: tester
titre: "Essayer l’aperçu et la sauvegarde du profil de face"
element: R-052
statut: fait
cree: 2026-11-13
par: agent
fait: 2026-11-19
---

Version à essayer : artifacts/FabSTEP.app (build de développement, distinct du paquet autonome 1.0.0). Si le bundle manque, reconstruire avec sh scripts/build-macos-dev.sh ; le lancement de développement est npm --prefix ui run desktop:dev.

Sélectionner une face STEP plane avec un trou, vérifier repère et aperçu 2D, exporter DXF puis changer de sélection et de fichier : aucun ancien profil ne doit rester exportable. Voir ADR-0024, V-1170/V-1171.

Répondre avec le résultat, le fichier et les réglages utilisés ; en cas de défaut, décrire les gestes, le résultat attendu et obtenu.

## Réponse

Accepté par l’utilisateur lors de la validation groupée des tests et des jalons du 19 novembre 2026.
