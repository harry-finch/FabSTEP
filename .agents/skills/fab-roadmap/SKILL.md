---
name: fab-roadmap
description: Feuille de route d'un projet au format de FabBoard (dossier roadmap/) — spécifier une idée avec l'utilisateur et la transformer en spécification détaillée, tenir les statuts à jour pendant le développement, créer une action quand on attend l'utilisateur (valider, tester, fournir, décider) et lire ses réponses, ajouter une idée, migrer une ancienne ROADMAP.md, terminer la mise d'équerre d'un projet (feuille de route, skills, documentation, figure de la carte). Utiliser dès qu'un projet contient roadmap/projet.toml, quand l'utilisateur parle de feuille de route, de roadmap, de jalon, de spécification, de « ce qu'il reste à faire », de mise d'équerre, ou qu'il lance /fab-roadmap. Pour Claude Code et Codex.
---

# Fab roadmap

La feuille de route d'un projet vit dans son dossier `roadmap/`, versionné avec le code. **FabBoard**, le tableau de bord de l'utilisateur, lit ces fichiers : ce que tu y écris, il le voit dans la minute, sur la carte du projet et dans sa liste « À moi de jouer ». C'est le canal entre toi et lui quand il n'est pas dans le terminal.

```
roadmap/
  projet.toml     fiche du projet : nom, jalons, versions à tester
  ordre.txt       priorité des éléments restants
  items/          un fichier par élément : R-012-export-step-par-dalle.md
  actions/        ce que l'UTILISATEUR doit faire : A-007-imprimer-le-vase.md
ROADMAP.md        vue générée : ne jamais la modifier à la main
```

Le format complet est dans [format.md](references/format.md) : le lire avant d'écrire un fichier à la main.

## La commande `fabboard`

À préférer à l'édition manuelle : elle attribue les identifiants, tient `ordre.txt`, régénère `ROADMAP.md`.

```sh
fabboard show . --json                                  # tout l'état : éléments, actions, diagnostics
fabboard roadmap add "Titre" --resume "Une ou deux phrases." [--jalon v1.0]
fabboard roadmap set R-012 --statut en-cours            # ou --titre, --resume, --jalon
fabboard roadmap move R-012 --avant R-009               # ou --apres, --debut, --fin
fabboard action add --type tester "Imprimer le vase" --element R-012 --corps "Où, quoi faire, quoi regarder, quoi répondre."
fabboard action list                                    # avec les réponses de l'utilisateur
fabboard roadmap lint                                   # avant chaque commit
fabboard roadmap render                                 # après une édition à la main
```

Le corps d'un élément (la spécification) s'écrit dans son fichier, sous l'en-tête, ou se passe par `--corps-fichier chemin.md`. Si `fabboard` n'est pas installé, écrire les fichiers à la main selon [format.md](references/format.md) et le dire à l'utilisateur.

Les fichiers de `roadmap/` et `ROADMAP.md` **se commitent avec le travail** qu'ils décrivent. L'option `--commit` existe sur chaque commande d'écriture, mais elle commite sur-le-champ, ces seuls fichiers : ne s'en servir que pour un changement isolé (une idée ajoutée, une action créée hors de tout travail en cours).

## Cycle de vie d'un élément

| Statut | Sens | Qui le pose |
| --- | --- | --- |
| `idee` | Un titre et une description rapide | L'utilisateur, ou toi sur sa demande |
| `specifiee` | Spécification détaillée, **validée par l'utilisateur** | Toi, après son accord explicite |
| `en-cours` | Le développement a commencé | Toi, en commençant |
| `a-tester` | Développé, attend sa vérification | Toi, en créant l'action qui dit quoi vérifier |
| `livree` | Vérifié et accepté par lui | Lui, ou toi sur son accord explicite |
| `abandonnee` | Retiré, le fichier reste | Lui |

Deux garde-fous : on ne développe pas un élément qui n'est pas `specifiee`, et on ne passe jamais soi-même un élément à `livree` parce que les tests sont verts.

## Au début de chaque session

1. `fabboard show . --json` (ou lire `ROADMAP.md`) : ce qui est en cours, à tester, et l'ordre de priorité.
2. `fabboard action list` : **lire les réponses** des actions faites depuis la dernière fois. Une réponse « ça marche » à une action `tester` autorise à passer l'élément à `livree` ; une réponse qui signale un défaut le ramène à `en-cours`, avec le défaut noté dans son corps.
3. Travailler sur le premier élément restant de l'ordre, sauf demande contraire.

## Spécifier une idée

Quand l'utilisateur lance « Spécifier avec l'agent » depuis FabBoard, ou demande de préciser un élément `idee` :

1. **Lire** l'élément, puis ce qui l'éclaire dans le projet : consignes d'agent, architecture, code concerné, éléments voisins et dépendances possibles.
2. **Questionner** l'utilisateur sur ce que le texte ne dit pas et que le code ne tranche pas : le besoin réel, les limites, ce qui est exclu, comment il vérifiera. Grouper les questions, proposer des options et une recommandation ; ne pas demander ce qu'on peut lire dans le dépôt.
3. **Rédiger** la spécification dans le corps du fichier, avec les six sections de [specification.md](references/specification.md).
4. **Faire valider** : montrer la spécification, corriger, obtenir un accord explicite. Si l'utilisateur n'est pas là, créer une action `valider` et s'arrêter : l'élément reste `idee`.
5. Après l'accord : `fabboard roadmap set R-… --statut specifiee`, ajuster `resume` et `jalon` si la discussion les a changés, `fabboard roadmap lint`, commit.

Une idée trop grosse pour une seule spécification se découpe en plusieurs éléments, avec l'accord de l'utilisateur.

## Pendant le développement

- En commençant : `--statut en-cours`.
- Si la spécification se révèle fausse ou incomplète : la corriger dans le fichier et le signaler ; un changement de périmètre demande l'accord de l'utilisateur (action `decider` s'il n'est pas là).
- Une fois développé et tes propres contrôles passés : `--statut a-tester`, et **une action par ligne** de « Vérifications de l'utilisateur » (type `tester` ou `valider`, rattachée par `--element`). Le corps de l'action dit où est la version à essayer, les gestes à faire, ce qu'il faut observer, et ce qu'on attend comme réponse.

## Quand tu attends l'utilisateur

Tout arrêt qui dépend de lui laisse une action, sinon il ne saura pas que c'est à lui :

| Type | Pour |
| --- | --- |
| `valider` | Une spécification, une proposition, un choix (logo, texte) |
| `tester` | Un build, une pièce fabriquée, un parcours dans l'application |
| `fournir` | Un fichier, une mesure, un accès, un document |
| `decider` | Un arbitrage entre des options que tu as décrites |

Un titre à l'infinitif, qui se comprend seul (« Choisir le logo parmi la planche »). Un corps qui permet d'agir sans rouvrir la conversation. Une seule demande par action.

## Ajouter une idée

`fabboard roadmap add "Titre" --resume "…"` : statut `idee`, en fin de liste. Ne pas spécifier dans la foulée sans qu'on te le demande. Une idée qui vient de toi en cours de travail (une dette, une suite évidente) s'ajoute ainsi, et se signale.

## Terminer la mise d'équerre d'un projet

Quand l'utilisateur lance « Terminer la mise d'équerre avec l'agent » depuis FabBoard, ou demande de mettre un projet d'équerre : suivre [equerre.md](references/equerre.md). Une session, quatre étapes : la feuille de route, les skills, la documentation, la figure de la carte (dont il choisit le concept), puis un bilan.

## Migrer une ancienne feuille de route

Quand FabBoard signale `roadmap.legacy` (un `ROADMAP.md` écrit à la main) : suivre [migration.md](references/migration.md). L'essentiel : rien n'est perdu, rien n'est inventé, l'ancien fichier est conservé, et l'utilisateur valide le résultat.

## Règles

1. `ROADMAP.md` est généré : toute modification passe par `roadmap/`.
2. Un identifiant (`R-…`, `A-…`) ne change jamais et n'est jamais réutilisé.
3. Ne pas inventer : une information absente se demande, ou reste absente et se signale.
4. `fabboard roadmap lint` sans erreur avant chaque commit.
5. Les textes de la feuille de route sont en français, concrets, sans jargon inutile : l'utilisateur les lit dans FabBoard, hors contexte.
6. Ne pas modifier l'ordre de priorité ni abandonner un élément sans qu'il le demande.
