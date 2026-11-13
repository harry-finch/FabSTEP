---
id: A-025
type: tester
titre: "Essayer les plans de coupe STEP et les poignées"
element: R-053
statut: a-faire
cree: 2026-11-13
par: agent
fait:
---

Version à essayer : artifacts/FabSTEP.app (build de développement, distinct du paquet autonome 1.0.0). Si le bundle manque, reconstruire avec sh scripts/build-macos-dev.sh ; le lancement de développement est npm --prefix ui run desktop:dev.

Choisir XY/XZ/YZ puis un plan de face, les corps et le décalage. Déplacer par flèche, tourner par anneaux, annuler avec Échap, vérifier aperçu et sauvegarde après geste en clair/sombre FR/EN. Vérifier qu’un changement de plan invalide immédiatement l’ancien export (V-1172 à V-1175).

Répondre avec le résultat, le fichier et les réglages utilisés ; en cas de défaut, décrire les gestes, le résultat attendu et obtenu.
