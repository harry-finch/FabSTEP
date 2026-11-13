---
id: A-027
type: tester
titre: "Essayer les coupes du maillage source dans le bureau"
element: R-054
statut: a-faire
cree: 2026-11-13
par: agent
fait:
---

Version à essayer : artifacts/FabSTEP.app (build de développement, distinct du paquet autonome 1.0.0). Si le bundle manque, reconstruire avec sh scripts/build-macos-dev.sh ; le lancement de développement est npm --prefix ui run desktop:dev.

Dans Source, préparer une coupe, déplacer/tourner le plan et exporter -mesh-section.dxf, y compris après refus de reconnaissance STEP. Vérifier prétraitements, corps, trous et préservation d’un STEP déjà valide. Tester les invalidations lors d’un changement de source (ADR-0026, V-1176).

Répondre avec le résultat, le fichier et les réglages utilisés ; en cas de défaut, décrire les gestes, le résultat attendu et obtenu.
