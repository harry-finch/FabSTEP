---
id: A-098
type: decider
titre: "Choisir quand recompiler FabSTEP après nettoyage"
statut: fait
cree: 2026-11-18
par: agent
fait: 2026-11-18
---

Le nettoyage A+B approuvé en A-097 est exécuté sur fabboard/nettoyage : 35 chemins déplacés à la Corbeille, projet de 43,3 à 2,6 Gio. Les deux exécutables FabSTEP et le helper ont des empreintes SHA-256 inchangées. Aucun fichier suivi supprimé ; formatage Rust et git diff --check réussis.

Décider si les tests et le build complets doivent être relancés maintenant ou reportés au prochain développement. La skill .agents/skills/fab-cleanup/SKILL.md, §4, dit : « Après un retrait de la catégorie A, le premier build est long : le dire avant de le lancer, et demander s’il faut le faire maintenant. » Recommandation : reporter pour garder le projet allégé ; cette option ne vaut pas validation des tests après nettoyage.

Si accord pour relancer : npm --prefix ui ci, puis sh scripts/check.sh ; pour le paquet de développement, sh scripts/build-macos-dev.sh. Cela peut durer plusieurs minutes et recréer plusieurs Gio de caches. Ne pas reconstruire ni remplacer le paquet release qualifié. Garder large-stl dans le target worker séparé, sans activer cette feature sur le bureau. Consigner les résultats réellement obtenus ; les essais graphiques restent distincts.

Répondre « reporter » ou « relancer maintenant ». Aucune recompilation n’est autorisée par le seul accord A+B de nettoyage.

## Réponse

L’utilisateur choisit explicitement « Reporter au prochain développement (recommandé) » dans la conversation. Tests et builds après nettoyage non exécutés ; les commandes de reprise sont conservées dans cette action.
