# Mettre la documentation d'un projet d'équerre

Procédure pour amener la documentation d'un projet existant au contrat de [document-contract.md](document-contract.md). C'est l'étape « Documentation » de la session de mise d'équerre lancée depuis FabBoard (voir [`equerre.md`](../../fab-roadmap/references/equerre.md) de `fab-roadmap`).

Principe : **changements minimaux**. Le projet a déjà une documentation et un style ; on comble les manques et on corrige ce qui est faux, on ne réécrit pas ce qui est bon. Rien ne s'invente : ce qui ne se lit ni dans le code ni dans les documents existants se demande à l'utilisateur.

## 0. Avant de commencer

1. `git status` : si l'arbre de travail a des modifications en cours, le dire à l'utilisateur et demander comment continuer. Ne jamais les embarquer dans un commit.
2. Travailler sur la branche de la session (`fabboard/mise-d-equerre`) ; ne pas pousser.
3. Lire, dans l'ordre : `AGENTS.md`, `README.md`, `STATUS.md`, `ROADMAP.md`, la liste de `docs/` et de `docs/adr/`, les 60 premières lignes de `DEVLOG.md`. Noter pour chaque document : présent, absent, ou présent mais hors contrat.
4. Relever la langue, les titres et le ton de chaque document : ils se conservent.

## 1. Contrôler

Remplir cette liste avant de modifier quoi que ce soit.

**Obligatoires**
- [ ] `README.md` existe et dit : ce qu'est le projet, comment le lancer, comment le développer.
- [ ] Les commandes du `README.md` et d'`AGENTS.md` sont vraies : les prérequis et les scripts existent (`package.json`, `Cargo.toml`, `Makefile`), le port est le bon.
- [ ] `AGENTS.md` existe, avec : la présentation du projet et ce qu'il faut lire avant de modifier ; « Skills à utiliser » ; « Feuille de route » ; « Règles » ; « Commandes ».
- [ ] `CLAUDE.md` existe et ne contient que `@AGENTS.md`.
- [ ] Les skills citées par `AGENTS.md` sont installées dans `.agents/skills/`, et celles qui s'imposent y sont citées (`fab-roadmap`, `project-documentation` ; `fab-design-system` si le projet a une interface).
- [ ] La section « Feuille de route » d'`AGENTS.md` renvoie à `fab-roadmap` et ne décrit pas le format.
- [ ] `STATUS.md` existe, tient en une page environ, sépare le vérifié du non vérifié, et ne porte pas de « prochaine action » pour l'utilisateur.
- [ ] `roadmap/projet.toml` existe ; `ROADMAP.md` porte le marqueur de génération ; `fabboard roadmap lint` ne signale rien. Sinon, c'est l'étape « Feuille de route » de la mise d'équerre (`fab-roadmap`), pas celle-ci.

**Cohérence**
- [ ] Un seul rôle par document : pas de récit recopié entre `STATUS.md`, `DEVLOG.md`, `README.md` et `docs/`.
- [ ] Aucun document ne cite un fichier, une commande ou un lien relatif qui n'existe plus. Vérifier les liens relatifs un par un.
- [ ] `STATUS.md` n'empile pas de bannières ni d'historique.
- [ ] Les affirmations de `STATUS.md` marquées « vérifié » disent comment elles l'ont été.
- [ ] Les ADR acceptés n'ont pas été réécrits ; le numéro suit sans trou ni doublon.

**Facultatifs** (`DEVLOG.md`, `docs/ARCHITECTURE.md`, `docs/adr/`, `docs/FEATURES.md`, `docs/VALIDATION.md`)
- [ ] Pour chacun de ceux qui existent : il a bien le rôle du contrat et n'est pas périmé.
- [ ] Ceux qui n'existent pas : ne rien créer.

## 2. Créer ce qui manque

Un document obligatoire absent se crée **à partir de ce qui existe**. Pour chaque fait écrit, savoir d'où il vient. Quand ce n'est pas le cas, poser la question à l'utilisateur ; ne pas deviner. Grouper les questions, en proposer une réponse quand le code en suggère une.

| Absent | Source | À demander à l'utilisateur |
| --- | --- | --- |
| `README.md` | Manifestes (`Cargo.toml`, `package.json`), `AGENTS.md`, arborescence, licence du dépôt | À qui s'adresse le projet, si ça ne se lit nulle part |
| `AGENTS.md` | `README.md`, `CLAUDE.md` s'il a du contenu (le déplacer dans `AGENTS.md`), manifestes pour les commandes, `.agents/skills/` pour les skills | Les règles de fond que le code ne montre pas |
| `CLAUDE.md` | — | Rien : une ligne, `@AGENTS.md`. S'il avait du contenu, le fondre dans `AGENTS.md` d'abord |
| `STATUS.md` | `ROADMAP.md` ou `roadmap/`, derniers commits et tags, résultat des tests lancés maintenant, `DEVLOG.md` | Ce qui a été essayé et validé à la main : sans réponse, l'écrire « non vérifié » |
| `roadmap/` | Voir `fab-roadmap` | — |

**Écrire `STATUS.md` sans rien inventer.**
1. Lancer les tests et les vérifications du projet (les commandes d'`AGENTS.md`). Écrire le résultat réel, avec la date. Si on ne peut pas les lancer, écrire « non lancé » et pourquoi.
2. Le jalon vient de `roadmap/projet.toml`, d'un tag ou du `README.md` ; sinon le demander.
3. Ce qu'on ne sait pas vérifier va dans « Non vérifié ».
4. Ne pas recopier la feuille de route : renvoyer aux éléments `R-…`.

**Compléter une section d'`AGENTS.md`.**
- « Skills à utiliser » : les skills réellement présentes dans `.agents/skills/`, chacune avec le moment de l'employer.
- « Feuille de route » : quelques lignes, par exemple :
  « La feuille de route est dans `roadmap/` (un fichier par élément, un par action de l'utilisateur), lue par FabBoard ; `ROADMAP.md` est généré, ne jamais le modifier à la main. Skill `fab-roadmap` : spécifier, tenir les statuts, créer une action quand on attend l'utilisateur, `fabboard roadmap lint` avant chaque commit. »
- « Règles » et « Commandes » : reprendre ce qu'`AGENTS.md`, le `README.md` ou le code disent déjà ; ne pas en ajouter d'autres.

## 3. Convertir la « prochaine action » de `STATUS.md`

Une section « Prochaine action » (ou équivalent : « À faire », « Suite », une ligne ⏸) mêle deux choses : ce que **l'utilisateur** doit faire, et ce que **l'agent** fera ensuite. Les séparer.

1. Lire la section en entier. Découper en phrases ; classer chacune :
   - attend l'utilisateur : tester, valider, fournir, décider → **une action** ;
   - travail pour l'agent, déjà dans la feuille de route → rien à créer, le renvoi `R-…` suffit ;
   - travail pour l'agent, absent de la feuille de route → **une idée** (`fabboard roadmap add`), sans la spécifier.
2. Vérifier avec `fabboard action list` et `fabboard show . --json` qu'une action ou un élément équivalent n'existe pas déjà. Ne pas créer de doublon.
3. Créer une action par demande, avec un titre à l'infinitif qui se comprend seul, et un corps qui permet d'agir sans rouvrir la conversation :

   ```sh
   fabboard action add --type tester "Essayer la v0.7 de FabVents" --element R-021 \
     --corps "Où : application de bureau et serveur MCP dans Claude Code. Quoi : liste du message de livraison. Répondre : ce qui marche, ce qui bloque."
   ```

   Types : `valider`, `tester`, `fournir`, `decider`. Détail et règles d'écriture : skill `fab-roadmap`.
4. Réécrire la section de `STATUS.md` : retirer la « prochaine action » et mettre à la place une ligne « À suivre » qui renvoie aux éléments et aux actions créés (« Actions en attente : A-007, A-008. En cours : R-021. »).
5. Une demande vague (« tester un peu ») : la préciser avec l'utilisateur, ou la créer telle quelle en le disant, avec une réponse attendue explicite.
6. Une action qui concerne un élément `idee` ou `specifiee` non encore livré se rattache à lui par `--element`.

## 4. Corriger sans réécrire

- Ne corriger que ce qui est **faux, périmé ou hors contrat** : une commande qui n'existe plus, un lien mort, une « prochaine action », un récit qui double un autre document, une section « Feuille de route » qui décrit le format au lieu de renvoyer.
- Garder la langue, les titres, le ton, la mise en page. Un `STATUS.md` en tableau le reste.
- Un récit qui double un autre document : garder la version du bon endroit et remplacer l'autre par un lien. Ne rien supprimer sans que le contenu existe ailleurs.
- Un `STATUS.md` trop long : déplacer l'historique dans `DEVLOG.md` si le projet en a un, sinon proposer à l'utilisateur de le créer. Ne jamais jeter l'historique.
- Ne pas toucher aux ADR acceptés, aux rapports datés, à `docs/ROADMAP-avant-fabboard.md` ni à `ROADMAP.md` (généré).
- Un document facultatif qui n'existe pas n'est pas un manque.

## 5. Montrer et faire valider

1. `git diff --stat` puis `git diff` : relire ce qui a changé.
2. Présenter à l'utilisateur, **document par document** : ce qui a été créé, ce qui a été modifié, ce qui a été laissé tel quel et pourquoi, et les questions restées ouvertes.
3. Obtenir son accord explicite. S'il corrige, reprendre. S'il n'est pas là : ne pas commiter, créer une action `valider` (« Valider la documentation de <projet> ») qui dit ce qui est prêt et où, et s'arrêter.
4. Après l'accord : un commit pour la documentation, message court (« Documentation : mise en conformité avec project-documentation »). Les fichiers de `roadmap/` créés pendant la conversion de la « prochaine action » se commitent avec elle. `fabboard roadmap lint` sans erreur avant le commit. Ne pas pousser.

## 6. Rapporter

À la fin, dire à l'utilisateur en quelques lignes :
- les documents **créés**, **modifiés**, **laissés** (avec la raison) ;
- les **actions** et les **éléments** créés depuis la « prochaine action » (identifiants et titres) ;
- ce qui est marqué « non vérifié » ou « à confirmer » faute de réponse ;
- les écarts qui subsistent et ce qu'ils demandent (un document facultatif absent qui aiderait, un historique trop long à déplacer) ;
- le nom de la branche et le commit, à fusionner par lui.

Ne rien rapporter qui n'ait été fait, ne pas présenter une hypothèse comme un fait.
