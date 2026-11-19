---
id: A-020
type: tester
titre: "Essayer les indications guidées successives et leur retrait"
element: R-047
statut: fait
cree: 2026-11-13
par: agent
fait: 2026-11-19
---

Version à essayer : artifacts/FabSTEP.app (build de développement, distinct du paquet autonome 1.0.0). Si le bundle manque, reconstruire avec sh scripts/build-macos-dev.sh ; le lancement de développement est npm --prefix ui run desktop:dev.

Utiliser une source avec plusieurs cylindres ou fixtures/guided_cylinder_instances.3mf. Ajouter deux indications, en retirer une et provoquer un refus ; utiliser Recalculer les indications conservées. Vérifier la session, tous les corps et l’absence d’export périmé. Voir ADR-0020.

Répondre avec le résultat, le fichier et les réglages utilisés ; en cas de défaut, décrire les gestes, le résultat attendu et obtenu.

## Réponse

Accepté par l’utilisateur lors de la validation groupée des tests et des jalons du 19 novembre 2026.
