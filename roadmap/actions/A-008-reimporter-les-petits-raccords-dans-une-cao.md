---
id: A-008
type: tester
titre: "Réimporter les petits raccords dans une CAO indépendante"
element: R-030
statut: a-faire
cree: 2026-11-13
par: agent
fait:
---

Version à essayer : artifacts/FabSTEP.app (build de développement, distinct du paquet autonome 1.0.0). Si le bundle manque, reconstruire avec sh scripts/build-macos-dev.sh ; le lancement de développement est npm --prefix ui run desktop:dev.

Ouvrir le STEP exporté dans Fusion ou FreeCAD et contrôler surfaces, dimensions et volume ; conserver les refus des cas incertains décrits en ADR-0022.

Répondre avec le résultat, le fichier et les réglages utilisés ; en cas de défaut, décrire les gestes, le résultat attendu et obtenu.
