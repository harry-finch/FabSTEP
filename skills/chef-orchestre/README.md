# Chef d'orchestre — Skill Codex

Skill transversale de coordination multi-agents pour les projets logiciels.

## Installation par projet

Copier le dossier entier dans :

```
<depot>/.agents/skills/chef-orchestre/
```

La skill est découverte par Codex depuis `.agents/skills`. Si elle n'apparaît pas immédiatement, redémarrer la session.

## Installation globale (facultative)

Copier le dossier dans :

```
~/.agents/skills/chef-orchestre/
```

Éviter les doublons global + projet portant le même nom si leur contenu diverge.

## Déclenchement

Dans Codex, mentionner `$chef-orchestre`, ou ajouter le court extrait fourni dans `examples/AGENTS-snippet.md` au `AGENTS.md` du projet.

Exemple :

> `$chef-orchestre` Implémente la fonction d'import STL. Clarifie uniquement les choix d'architecture structurants, décompose le travail, utilise les sous-agents adaptés, vérifie leurs rapports et fais une revue finale avec preuves de tests.

## Prérequis et limites

- Codex doit disposer de la fonctionnalité sous-agents pour une réelle exécution multi-agent.
- Les modèles accessibles et la possibilité de choisir le niveau de raisonnement dépendent du client et de sa configuration.
- La skill définit une politique de choix ; elle ne crée pas à elle seule un moteur de planification, un ordonnanceur autonome ou des modèles custom.
- Un surcroît de sous-agents peut augmenter les tokens consommés ; aucune économie n'est garantie.
- La skill ne lance pas automatiquement de push, de release ou de déploiement.

Références officielles : https://learn.chatgpt.com/docs/build-skills et https://learn.chatgpt.com/docs/agent-configuration/subagents
