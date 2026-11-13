# Contrat de documentation

Un document, un rôle. Pour chaque document : son rôle, ce qu'il contient, ce qu'il ne contient pas, sa structure, qui le met à jour et quand. Les exemples viennent de FabVents et de FabWeave.

Tableau d'ensemble dans [SKILL.md](../SKILL.md). Les documents obligatoires viennent d'abord ; les facultatifs se tiennent seulement s'ils existent ou si l'utilisateur les demande.

La langue du projet se conserve (les projets Fab sont en français : textes, titres, commentaires). Les identifiants de code restent ceux du code.

---

## Obligatoires

### README.md

**Rôle.** La porte d'entrée : à quoi sert le projet, comment l'essayer et le développer. Il s'adresse à quelqu'un qui découvre le dépôt, humain ou agent.

**Contient.**
- Un titre et un paragraphe qui dit ce que fait le projet, pour qui, et en quoi il est dans l'écosystème Fab.
- Une ligne d'état courte qui renvoie à `STATUS.md` (« État : v0.7 livrée pour essai. Voir [STATUS.md](STATUS.md). »).
- Comment le lancer : prérequis (versions), commandes exactes, adresse locale.
- Comment le développer : tests, vérifications, build.
- L'organisation du dépôt (tableau « dossier, rôle »), si le dépôt a plusieurs parties.
- La licence.

**Ne contient pas.** L'historique, l'état détaillé, la feuille de route, la liste des fonctions : des liens, pas des copies.

**Structure type** (FabVents, FabWeave) : `# Nom` · présentation · état · `## Lancer` · `## Utiliser` (si utile) · `## Développer` · `## Organisation` · `## Licence`.

**Mise à jour.** Par celui qui change une commande, un prérequis, un port ou l'organisation du dépôt, dans le même changement. La ligne d'état suit chaque jalon.

### AGENTS.md et CLAUDE.md

**Rôle.** Les consignes que lit un agent de code (Claude Code ou Codex) avant de toucher au projet. `CLAUDE.md` ne contient que `@AGENTS.md` : une seule source, deux lecteurs.

**Contient.** Les cinq blocs ci-dessous, dans cet ordre.

**Ne contient pas.** L'état du projet (`STATUS.md`), la description de la feuille de route (`fab-roadmap`), un récit, une règle déjà garantie par un outil (le formateur, le compilateur).

**Structure attendue.**

```markdown
# Consignes pour les agents

> Ce fichier est repris dans CLAUDE.md.

<Ce qu'est le projet, en deux ou trois phrases : nature, pile technique, place dans la famille.
Lire [README.md](README.md), [STATUS.md](STATUS.md), [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
avant de modifier.>

## Skills à utiliser (`.agents/skills/`)

- `fab-design-system` : toute modification de l'interface…
- `fab-roadmap` : …
- `project-documentation` : …

## Feuille de route

<Quelques lignes : elle est dans `roadmap/`, lue par FabBoard ; `ROADMAP.md` est généré, ne jamais
le modifier à la main ; renvoi à la skill `fab-roadmap` pour spécifier, tenir les statuts et créer
une action quand on attend l'utilisateur ; `fabboard roadmap lint` avant chaque commit.>

## Règles

- <Règles propres au projet, une par ligne, vérifiables.>
- Mettre à jour la documentation dans le même changement que le comportement.

## Commandes

- Tests : `cargo test --workspace`, …
- Lancement : `cd ui && npm run dev` → http://localhost:5173
- Build : `cd ui && npm run build`
```

Ce que font FabVents et FabWeave :
- Le premier paragraphe liste ce qu'il faut lire avant de modifier, avec des liens.
- Chaque skill citée a une phrase qui dit **quand** l'employer (« toute modification de l'interface »).
- Les règles sont des invariants du projet, pas des conseils : `fabvents-core` reste pur (aucune IO) ; millimètres et degrés dans les fichiers ; valeurs hors domaine signalées, jamais corrigées en silence ; identifiants de `FEATURES.md` stables.
- Les commandes sont copiables telles quelles et disent d'où les lancer.

**Mise à jour.** Par celui qui ajoute une règle de fond, une skill, une commande ou un document à lire. Une règle nouvelle découverte en travaillant (« ne jamais faire X, parce que Y ») s'y ajoute, sans attendre.

### STATUS.md

**Rôle.** Répondre en une minute à « où en est le projet, et qu'est-ce qui est vérifié ? ». C'est l'état d'aujourd'hui, pas l'histoire.

**Contient.**
- Le jalon en cours, sa date, le dernier jalon validé par l'utilisateur.
- L'état par sujet, en distinguant ce qui est **vérifié** (et comment : test, commande, essai dans le navigateur, essai de l'utilisateur) de ce qui est **non vérifié**.
- Les points ouverts : limites connues, défauts, dettes.
- Des renvois aux éléments de la feuille de route (`R-012`) et aux actions en attente (`A-007`).

**Ne contient pas.**
- De « prochaine action » pour l'utilisateur : ce qu'il doit faire vit dans `roadmap/actions/`.
- La chronologie (`DEVLOG.md`), la spécification d'un élément (son fichier dans `roadmap/items/`), la liste complète des fonctions (`FEATURES.md`).
- Des bannières empilées (« mise à jour du… ») : on corrige la ligne en place.

**Structure attendue** (celle de FabVents et de FabWeave).

```markdown
# État du projet

**Jalon** : **v0.7 — bureau et automatisation**, livrée pour essai (07/11/2026).
v0.1 à v0.6 validées par l'utilisateur. Suite : voir [ROADMAP.md](ROADMAP.md).

| Élément | État |
| --- | --- |
| <Sujet> | <Ce qui existe, en une ou deux phrases> |
| Tests | `cargo test --workspace` : 151 verts ; `npm run check` : 0 erreur |
| Vérifié | <Ce qui a été essayé, et où : navigateur, CLI, impression> |
| Non vérifié | <Ce qui ne l'a pas été> |

## Points ouverts

- <Limite connue ou défaut, une ligne.>

## À suivre

- Éléments en cours ou à tester : R-012, R-015. Actions en attente : A-007.
```

Garder les lignes « Tests », « Vérifié » et « Non vérifié » : ce sont elles qui séparent les faits des hypothèses. Une cellule de tableau reste lisible en deux lignes ; au-delà, c'est du `DEVLOG.md`.

**Mise à jour.** Par l'agent, à chaque arrêt et à chaque jalon, **en place** : on remplace une ligne devenue fausse, on ne l'empile pas. Une ligne « Non vérifié » passe à « Vérifié » quand l'utilisateur ou un essai l'a établi, avec la date.

### roadmap/ et ROADMAP.md

**Rôle.** La feuille de route et les actions de l'utilisateur. Tout est décrit par la skill [`fab-roadmap`](../../fab-roadmap/SKILL.md) et son [format](../../fab-roadmap/references/format.md) : ne rien en répéter ici.

**Pour ce contrat.** Le dossier `roadmap/` existe (avec `projet.toml`) ; `ROADMAP.md` porte le marqueur de génération et ne se modifie jamais à la main ; `AGENTS.md` a sa section « Feuille de route ». Une ancienne feuille de route écrite à la main est conservée (FabVents et FabWeave : `docs/ROADMAP-avant-fabboard.md`).

---

## Facultatifs

À tenir seulement s'ils existent ou si l'utilisateur les demande. S'ils existent, ils suivent ce qui suit.

### DEVLOG.md

**Rôle.** Le journal chronologique : ce qu'une session a fait, choisi, vérifié. Il porte l'histoire qu'on retire de `STATUS.md`.

**Contient.** Une entrée par session ou par étape, la plus récente en dernier ou en premier selon l'usage du projet (le conserver). Titre `## AAAA-MM-JJ — sujet`, puis les rubriques utilisées par le projet : **Entrées** (ce qui a été lu ou reçu), **Choix** (avec renvoi à l'ADR), **Vérification** (ce qui a été mesuré ou essayé), **Défauts corrigés**, **Non fait**.

**Ne contient pas.** Le bruit de terminal, une décision de fond sans ADR, l'état actuel.

**Mise à jour.** On ajoute à la fin d'une session ou d'une étape ; on ne réécrit pas les entrées passées.

### docs/ARCHITECTURE.md

**Rôle.** Les frontières stables et le flux de données : ce qu'un agent doit comprendre avant de modifier.

**Contient.** Une vue d'ensemble (schéma), les crates ou modules et leur responsabilité, le flux des données, les invariants. Les choix justifiés renvoient aux ADR.

**Ne contient pas.** Le détail d'implémentation qu'on lit dans le code et qui vieillit vite.

**Mise à jour.** Quand une frontière, un flux ou un module change, dans le même changement.

### docs/adr/

**Rôle.** Une décision d'architecture par fichier : `0005-trous-ame-et-arrondi.md`. Gabarit : [ADR-TEMPLATE.md](../assets/ADR-TEMPLATE.md).

**Contient.** Contexte, décision, alternatives envisagées, conséquences, validation. Statuts : Proposée, Acceptée, Remplacée par ADR-XXXX, Abandonnée.

**Règles.**
- Numéro à quatre chiffres, suivant le dernier ; jamais réutilisé.
- Un ADR **accepté** ne se réécrit pas, sauf pour changer son statut ou corriger une faute. Une décision qui change : nouvel ADR qui remplace l'ancien.
- Un ADR `Proposée` peut être modifié jusqu'à l'accord de l'utilisateur ; c'est lui qui l'accepte.
- Écrire un ADR pour un choix qui engage la suite (format de fichier, frontière entre modules, bibliothèque structurante), pas pour chaque détail.

**Mise à jour.** À la décision, avant ou avec le code.

### docs/FEATURES.md

**Rôle.** L'inventaire des fonctions, avec des identifiants stables (`LAY-03`) qui permettent d'en parler sans ambiguïté.

**Contient.** Des tableaux par famille : identifiant, fonction, jalon, état (**fait**, **en cours**, **prévu**, **hors périmètre**).

**Ne contient pas.** La priorité ni l'ordre (feuille de route) ; l'état global (`STATUS.md`).

**Mise à jour.** La feuille de route fait foi pour l'avancement : l'état d'une fonction ne change qu'à la livraison de l'élément `R-…` qui la couvre, dans le même changement que son code. Un identifiant ne change ni ne se réutilise.

### docs/VALIDATION.md

**Rôle.** Les cas de validation et leur état : ce qui a été essayé contre une référence ou en réel (pièce découpée, impression, import dans un autre logiciel).

**Contient.** Un identifiant durable par cas, ce qu'on vérifie, la référence, le résultat, la date, l'état (réussi, échoué, non essayé). Un projet peut aussi en tenir plusieurs dans `docs/validation/` (FabVents : `step-freecad.md`).

**Ne contient pas.** L'état global (`STATUS.md`), la chronologie (`DEVLOG.md`).

**Mise à jour.** Quand un cas est essayé. Un rapport daté ne se réécrit pas : un nouvel essai s'ajoute.

---

## Autres documents

Un projet peut avoir d'autres documents propres (FabWeave : `docs/WEAVE_FORMAT.md`, `docs/PROMPT_MODELE_IA.md`, `docs/AUDIT-01.md` ; FabBoard : `docs/FORMAT.md`, `docs/DESIGN.md`). Ils gardent leur rôle : `AGENTS.md` les cite avec le moment où les lire. Ne pas les fusionner dans le contrat ni les renommer.

## Ordre de confiance

En cas de contradiction : le code et les contrats générés, puis les ADR acceptés, puis les essais et les tests qui les prouvent, puis la prose ancienne. Corriger la prose ; ne pas l'empiler sous une bannière.
