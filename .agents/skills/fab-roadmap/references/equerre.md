# Terminer la mise d'équerre d'un projet

FabBoard met un projet d'équerre en deux temps. Le premier est automatique : il crée `roadmap/`, remplit `projet.toml` de ce qui se détecte, installe cette skill. Le second demande du jugement et l'avis de l'utilisateur : c'est cette session, lancée depuis FabBoard par « Terminer la mise d'équerre avec l'agent ».

À la fin, le projet a une feuille de route au format, une documentation conforme, des skills à jour, une figure pour sa carte, et FabBoard n'a plus rien à signaler que l'utilisateur ne sache.

## Avant de commencer

1. `fabboard doctor .` : la liste des écarts est le programme de la session. S'il n'y a pas de dossier `roadmap/`, s'arrêter : c'est le bouton « Mettre d'équerre » de FabBoard qui le crée.
2. `git status` : si l'arbre de travail a des modifications en cours, le dire et demander s'il faut continuer ; ne jamais les embarquer dans un commit.
3. Travailler sur une branche dédiée (`fabboard/mise-d-equerre`) sauf demande contraire. Ne pas pousser.
4. Annoncer à l'utilisateur les étapes qui le concernent, pour qu'il sache quand on aura besoin de lui : les skills à remplacer, la documentation à valider, le concept de la figure à choisir.

Les étapes sont indépendantes : en sauter une déjà faite, et faire **un commit par étape**.

### Si la session ne peut pas écrire dans `.git/` ou `.agents/`

Certains agents tournent dans un bac à sable qui protège ces deux dossiers (Codex en `workspace-write` sans demande d'approbation, par exemple) : créer une branche, commiter ou mettre une skill à jour échoue alors (« cannot lock ref », « Operation not permitted »). Ne pas insister ni contourner.

- Le vérifier **d'emblée** (`git switch -c fabboard/mise-d-equerre`) et le dire à l'utilisateur avant de commencer, plutôt qu'à la fin.
- Faire tout le reste : les fichiers de `roadmap/`, la documentation et la figure s'écrivent normalement.
- Pour les skills : indiquer à l'utilisateur les boutons « Remplacer… », « Garder » et « Mettre à jour » de la fiche du projet dans FabBoard, qui font la même chose que les commandes.
- À la fin, **une seule** action `fournir` qui donne les commandes exactes à lancer par lui (les `git add` et `git commit` par étape, les `fabboard skills …`), dans l'ordre. Ne pas créer d'action de validation tant que cela n'est pas fait.

## 1. Feuille de route

Si `fabboard doctor` signale `roadmap.legacy` : suivre [migration.md](migration.md) en entier.

Si `fabboard doctor` signale `roadmap.empty` (le dossier existe, sans aucun élément) et que le projet a déjà une histoire : la **reconstituer**, voir « Projet sans feuille de route » plus bas. Un projet tout neuf n'a rien à reconstituer : demander à l'utilisateur ses premières idées et les ajouter (`fabboard roadmap add`).

Dans tous les cas, vérifier que `projet.toml` dit vrai : sous-titre, couleur, logo, jalons, et surtout les **versions à tester** (`[[test]]`) : l'adresse locale et la commande qui la sert, le motif des builds, la release. Demander à l'utilisateur comment il essaie le projet si rien ne se déduit des consignes d'agent. Renseigner `agent` (`claude` ou `codex`) s'il le dit.

### Projet sans feuille de route

Un projet avancé, voire presque terminé, peut n'avoir jamais eu de feuille de route : FabBoard affiche alors « Feuille de route à écrire » au lieu de son avancement réel. Le but est que la carte dise vrai : ce qui est livré, ce qui reste.

1. **Lire ce qui existe**, dans cet ordre : les versions publiées (`git tag`, notes de version, `CHANGELOG`), `STATUS.md` et `DEVLOG.md`, le `README.md` (ce que l'application fait), `docs/FEATURES.md` s'il existe, puis l'historique Git (`git log --oneline`) pour dater.
2. **Proposer** à l'utilisateur, avant d'écrire quoi que ce soit, une feuille de route à gros grain :
   - les **jalons** : une version publiée ou un grand lot par jalon, dans l'ordre ;
   - **ce qui est livré** : un élément `livree` par jalon ou par grand lot (pas un par fonction), avec un titre, une description rapide tirée des documents, et la date du tag ou du commit ;
   - **ce qui reste** : cela ne se devine pas. Lui demander ce qui manque pour qu'il considère le projet terminé, et ce qu'il envisage ensuite ; chaque réponse devient une idée.
3. Après son accord seulement : `[[jalons]]` dans `projet.toml`, puis les éléments (`fabboard roadmap add … --cree`, `fabboard roadmap set … --statut livree --livree`), le corps de chaque élément livré citant sa source (« d'après les notes de la version 0.9.0 »). `ordre.txt` reprend la priorité qu'il a donnée à ce qui reste.
4. Si une version publiée attend encore son essai, l'élément est `a-tester` et une action `tester` le dit.

Ne rien inventer : pas de jalon, de date ni de fonction qui ne soit dans les documents, l'historique ou ses réponses. Une feuille de route reconstituée de cinq éléments justes vaut mieux qu'une de trente supposés.

## 2. Skills

`fabboard skills status` donne l'état de chaque skill du projet par rapport à la bibliothèque.

- `en-retard` : `fabboard skills install <nom>` les met à jour, sans risque.
- `differente` (copie d'avant FabBoard) ou `modifiee` : montrer à l'utilisateur ce qui diffère (`diff -r` entre la copie du projet et celle de la bibliothèque, résumé en quelques lignes par fichier), et dire si ce que le projet a en propre existe déjà dans la bibliothèque. Puis, selon sa réponse :
  - remplacer par la version de la bibliothèque : `fabboard skills install <nom> --force` ;
  - garder la copie du projet telle quelle : `fabboard skills keep <nom>` (FabBoard cesse de la signaler) ;
  - si le projet a une retouche que la bibliothèque n'a pas et qui mérite d'y aller : ne pas l'écraser, la signaler, c'est à lui de décider où elle va.
- `hors-bibliotheque` : ne pas y toucher.
- Une skill que les consignes d'agent citent et qui n'est pas installée : proposer de l'installer.

## 3. Documentation

Suivre `references/mise-en-conformite.md` de la skill `project-documentation` (l'installer si elle manque : `fabboard skills install project-documentation`). En bref : `README.md`, `AGENTS.md` repris par `CLAUDE.md`, `STATUS.md` et `roadmap/` sont attendus ; ce qui manque se crée à partir du code et des documents existants, sans rien inventer ; ce qui est bon ne se réécrit pas ; la « prochaine action » d'un `STATUS.md` devient des actions de l'utilisateur ; les consignes d'agent renvoient à cette skill pour la feuille de route.

Montrer les changements et obtenir l'accord de l'utilisateur avant de commiter.

## 4. Figure de la carte

Si `fabboard doctor` signale `project.no_figure`, ou si `projet.toml` porte une figure marquée provisoire : le projet n'a pas encore sa **figure du métier**, l'illustration au trait de sa carte dans FabBoard.

1. Lire la skill `fab-illustration-system` (l'installer si elle manque), en particulier `references/creer-et-adopter.md` et le tableau « Ce que le catalogue n'a pas » de `references/catalogue.md` : il donne un point de départ pour plusieurs applications.
2. Installer `hairline-create` dans le projet si elle n'y est pas, comme le dit `fab-illustration-system`.
3. **Proposer deux ou trois concepts** : pour chacun, l'objet du métier dessiné, ce que le pointeur y règle, et pourquoi il dit ce que fait l'application. **Le choix revient à l'utilisateur**, comme pour un logo. Ne rien dessiner avant.
4. Dessiner la figure choisie avec `hairline-create`, en suivant ses règles et ses contrôles (validation du fichier, regard dans un navigateur). Ne jamais dessiner une figure à la main ni modifier le moteur.
5. La ranger dans `branding/illustrations/<nom>.js`, avec le `README.md` que `fab-illustration-system` demande. FabBoard la lit à cet endroit : rien à copier ailleurs.
6. `figure = "<nom>"` dans `roadmap/projet.toml` (retirer le commentaire « provisoire »).
7. Demander à l'utilisateur s'il veut aussi la figure **dans l'application** (état vide, premier lancement, À propos) : c'est un autre travail, décrit par `fab-illustration-system`. S'il le veut, l'ajouter à la feuille de route (`fabboard roadmap add`), ne pas le faire dans cette session.

Si l'utilisateur n'est pas là pour choisir : créer une action `valider` (« Choisir le concept de la figure de <projet> ») avec les concepts dans son corps, et laisser la figure provisoire.

## 5. Bilan

1. `fabboard doctor .` et `fabboard roadmap lint` : ce qui reste est soit expliqué, soit devenu une action.
2. Une action `valider`, créée avec `--motif equerre` : « Valider la mise d'équerre de <projet> » (`fabboard action add --type valider --motif equerre "Valider la mise d'équerre de <projet>" --corps "…"`). Quand l'utilisateur la cochera, FabBoard notera le projet comme d'équerre. Dans son corps : ce qui a été fait à chaque étape, ce qui a été laissé et pourquoi, et la branche à fusionner.
3. Dire à l'utilisateur, en quelques lignes, ce qui a changé et ce qu'il lui reste à faire. Ne pas fusionner ni pousser : c'est sa décision.
