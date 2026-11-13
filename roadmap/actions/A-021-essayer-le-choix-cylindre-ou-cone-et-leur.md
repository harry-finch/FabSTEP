---
id: A-021
type: tester
titre: "Essayer le choix Cylindre ou Cône et leur enchaînement"
element: R-048
statut: a-faire
cree: 2026-11-13
par: agent
fait:
---

Version à essayer : artifacts/FabSTEP.app (build de développement, distinct du paquet autonome 1.0.0). Si le bundle manque, reconstruire avec sh scripts/build-macos-dev.sh ; le lancement de développement est npm --prefix ui run desktop:dev.

Sur les facettes d’un STEP Plans, essayer un tronc de cône à deux contours plans, puis un cylindre indépendant. Retirer chaque indication et vérifier récupération après refus. Les pointes et raccords courbes doivent rester refusés. Voir ADR-0027 et V-1177.

Répondre avec le résultat, le fichier et les réglages utilisés ; en cas de défaut, décrire les gestes, le résultat attendu et obtenu.
