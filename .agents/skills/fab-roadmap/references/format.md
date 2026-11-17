# Format de suivi d'un projet (version 1)

Ce document est le **contrat** entre FabBoard, les agents (Claude Code, Codex) et l'utilisateur. Tout ce que FabBoard sait de la feuille de route d'un projet est écrit dans le dossier `roadmap/` de ce projet, versionné avec lui. FabBoard ne garde chez lui que la liste des projets suivis.

Les clés des fichiers sont en français, sans accent. Les dates sont au format `AAAA-MM-JJ`.

## Arborescence

```
<projet>/
  ROADMAP.md              vue générée, lisible sur GitHub (ne pas modifier à la main)
  roadmap/
    projet.toml           fiche d'identité du projet
    intro.md              texte libre repris en tête de ROADMAP.md (facultatif)
    ordre.txt             ordre de priorité des éléments restants
    items/
      R-012-export-step-par-dalle.md
    actions/
      A-007-imprimer-le-vase.md
```

## `projet.toml`

```toml
format = 1
nom = "FabVents"
resume = "Motifs de perforation"
identite = "#B5D93A"                    # couleur d'identité (thème sombre)
logo = "ui/public/fabvents-logo.svg"    # chemin relatif à la racine du projet
figure = "plaque"                       # illustration de la carte, voir FabBoard
agent = "claude"                        # claude | codex : agent ouvert par FabBoard

# Versions à tester, de la plus pertinente à la moins pertinente.
[[test]]
type = "url"                            # une adresse servie par un serveur local
titre = "Version web"
url = "http://localhost:5173"
commande = "npm run dev"                # facultatif : ce qui sert l'adresse
dossier = "ui"                          # facultatif : où lancer la commande

[[test]]
type = "build"                          # le fichier le plus récent qui répond au motif
motif = "dist-desktop/*.dmg"

[[test]]
type = "release"                        # dernière release GitHub du dépôt distant

# Jalons, dans l'ordre. Facultatif.
[[jalons]]
id = "v1.0"
titre = "Publication"

# Dossiers reconstructibles, comptés comme « récupérables ». Facultatif :
# par défaut target, node_modules, dist, .venv, __pycache__, .svelte-kit.
[tailles]
recuperable = ["target", "ui/node_modules", "ui/dist"]
```

Seuls `format` et `nom` sont obligatoires. Une clé inconnue est conservée et signalée par le contrôle, jamais supprimée.

## Un élément de feuille de route

Un fichier par élément : `roadmap/items/<id>-<titre-court>.md`. L'identifiant (`R-` suivi de trois chiffres au moins) est stable, jamais réutilisé, et le nom du fichier commence par lui.

```markdown
---
id: R-012
titre: "Export STEP par dalle"
resume: "Découper la plaque en dalles imprimables et exporter un STEP par dalle."
statut: specifiee
jalon: v0.8
cree: 2026-11-11
maj: 2026-11-12
livree:
---

## But
…
```

| Clé | Obligatoire | Valeur |
| --- | --- | --- |
| `id` | oui | `R-012` |
| `titre` | oui | une ligne |
| `resume` | oui | la description rapide, une ou deux phrases |
| `statut` | oui | voir ci-dessous |
| `jalon` | non | identifiant d'un jalon de `projet.toml` |
| `cree`, `maj` | oui | dates de création et de dernière modification |
| `livree` | si `statut: livree` | date de livraison |
| `avant` | non | statut d'avant l'abandon ; écrit à l'abandon, retiré à la restauration, qui rend ce statut (un élément livré garde sa date `livree`) |

L'en-tête est du YAML restreint : une clé par ligne, une valeur scalaire sur une ligne. FabBoard écrit `titre` et `resume` entre guillemets doubles ; il lit aussi les formes sans guillemets. Le corps est du Markdown, conservé à l'octet près quand FabBoard ne modifie que l'en-tête.

### Cycle de vie

| `statut` | Affiché | Sens | Qui le pose |
| --- | --- | --- | --- |
| `idee` | Idée | Un titre et une description rapide ; le corps peut être vide ou contenir des notes | L'utilisateur (formulaire) ou un agent |
| `specifiee` | Spécifiée | La spécification détaillée a été validée par l'utilisateur avec l'agent | La skill de spécification |
| `en-cours` | En cours | Le développement a commencé | L'agent qui développe |
| `a-tester` | À tester | Développé ; attend une vérification de l'utilisateur (une action `tester` ou `valider` l'accompagne) | L'agent qui développe |
| `livree` | Livrée | Vérifié et accepté par l'utilisateur | L'utilisateur, ou l'agent sur son accord explicite |
| `abandonnee` | Abandonnée | Retiré de la feuille de route ; le fichier reste, pour pouvoir revenir en arrière | L'utilisateur (« supprimer ») |

**Avancement** = éléments livrés ÷ (tous les éléments − abandonnés), pour le projet et pour chaque jalon. Le **jalon en cours** est le premier jalon de `projet.toml` qui a encore un élément non livré.

### Spécification

À partir de `specifiee`, le corps contient ces sections, dans cet ordre (titres de niveau 2, exacts) :

1. `## But` : le besoin, en deux à quatre phrases.
2. `## Inclus` : ce qui sera fait, en liste.
3. `## Hors périmètre` : ce qui ne sera pas fait, pour qu'on ne le découvre pas à la livraison.
4. `## Dépendances` : éléments (`R-…`), décisions ou documents requis ; « Aucune » si rien.
5. `## Critères d'acceptation` : des faits observables et vérifiables.
6. `## Vérifications de l'utilisateur` : ce que l'utilisateur devra tester ou valider lui-même ; chaque ligne deviendra une action.

D'autres sections peuvent suivre (notes, décisions, maquettes).

Le contrôle signale **une fois par élément** les sections manquantes, pour les statuts `specifiee` et `en-cours` : c'est là que la spécification sert. Un élément `a-tester` ou `livree` n'y est pas tenu : les jalons développés avant l'adoption du format gardent leur texte d'origine.

## `ordre.txt`

Un identifiant par ligne, du plus prioritaire au moins prioritaire ; `#` commence un commentaire. Il ordonne les éléments **restants** (ni livrés ni abandonnés). Un élément restant absent du fichier est rangé à la fin, par identifiant ; un identifiant inconnu est ignoré. Les deux cas sont signalés par le contrôle.

Changer une priorité ne modifie que ce fichier.

## Une action de l'utilisateur

Ce que **l'utilisateur** doit faire pour que le projet avance. Un fichier par action : `roadmap/actions/<id>-<titre-court>.md` (`A-` suivi de trois chiffres au moins).

```markdown
---
id: A-007
type: tester
titre: "Imprimer le vase sur la CORE One"
element: R-012
statut: a-faire
cree: 2026-11-11
par: agent
fait:
---

Où est le build, quoi faire, quoi regarder, quoi répondre.

## Réponse

Ce que l'utilisateur a constaté (écrit par FabBoard quand il coche l'action).
```

| Clé | Obligatoire | Valeur |
| --- | --- | --- |
| `id` | oui | `A-007` |
| `type` | oui | `valider` (une proposition, une spécification, un choix de logo), `tester` (un build, une pièce), `fournir` (un document, un fichier, une mesure), `decider` (un arbitrage) |
| `titre` | oui | une ligne, à l'infinitif |
| `element` | non | l'élément de feuille de route concerné |
| `statut` | oui | `a-faire`, `fait`, `annulee` |
| `cree` | oui | date |
| `par` | oui | `agent` ou `utilisateur` |
| `fait` | si `statut: fait` | date |

Un agent qui s'arrête pour attendre l'utilisateur **crée une action** ; à la reprise, il lit les réponses des actions faites.

## `ROADMAP.md`

Vue générée depuis `roadmap/` : l'introduction, puis les éléments restants dans l'ordre de priorité (titre, description rapide, statut, lien vers le fichier), les actions à faire, les éléments livrés par jalon, et les éléments écartés (abandonnés). La première ligne est le marqueur :

```markdown
<!-- Généré par FabBoard depuis roadmap/ : ne pas modifier à la main. -->
```

Un `ROADMAP.md` sans ce marqueur est une ancienne feuille de route à migrer ; FabBoard ne l'écrase jamais.

## Qui écrit quoi

- **FabBoard** écrit dans `roadmap/` et régénère `ROADMAP.md` quand l'utilisateur ajoute, réordonne, abandonne ou restaure un élément, ou coche une action. Il **commite seul ces fichiers** (`git commit` limité à `roadmap/` et `ROADMAP.md`, message `roadmap: …`), sans jamais toucher au reste de l'arbre de travail ni à l'index de l'utilisateur, **sans jamais pousser**. Pas de commit pendant un rebase ou une fusion.
- **Les agents** modifient les mêmes fichiers à la main ou par la ligne de commande `fabboard`, et les commitent avec leur travail.
- La ligne de commande `fabboard roadmap lint` contrôle le format ; `fabboard roadmap render` régénère `ROADMAP.md`.

## Évolution

`format = 1`. Toute évolution incompatible augmente ce numéro et vient avec une migration et un test de lecture de l'ancien format.
