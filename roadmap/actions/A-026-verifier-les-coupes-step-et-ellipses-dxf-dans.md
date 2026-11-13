---
id: A-026
type: tester
titre: "Vérifier les coupes STEP et ellipses DXF dans Fusion"
element: R-053
statut: a-faire
cree: 2026-11-13
par: agent
fait:
---

Version à essayer : artifacts/FabSTEP.app (build de développement, distinct du paquet autonome 1.0.0). Si le bundle manque, reconstruire avec sh scripts/build-macos-dev.sh ; le lancement de développement est npm --prefix ui run desktop:dev.

Exporter une coupe droite et une coupe oblique elliptique. Dans Fusion vérifier mm, dimensions, trous, boucles et ellipses exactes ; les tangences/courbes non prises en charge restent des refus explicites, sans export partiel.

Répondre avec le résultat, le fichier et les réglages utilisés ; en cas de défaut, décrire les gestes, le résultat attendu et obtenu.
