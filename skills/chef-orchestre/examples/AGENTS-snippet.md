# Extrait facultatif à ajouter au AGENTS.md du dépôt

```markdown
## Orchestration des tâches de développement

Pour les tâches de développement non triviales, utiliser la skill
`chef-orchestre` disponible dans `.agents/skills/chef-orchestre/SKILL.md`.
L'agent principal reste responsable de l'architecture, de l'intégration et
de la vérification. Déléguer de façon proportionnée à la complexité et
au coût total en tokens. Ne jamais lancer d'écritures concurrentes sur les
mêmes fichiers. Solliciter l'utilisateur pour les clarifications bloquantes
et les décisions structurelles, pas pour les détails routiniers.
```

Ne pas remplacer les autres instructions AGENTS.md. La skill peut aussi être invoquée explicitement avec `$chef-orchestre`.
