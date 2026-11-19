---
name: fab-cleanup
description: Nettoyer un projet — retirer ce qui ne sert plus (dossiers reconstructibles, résidus non suivis, fichiers suivis devenus inutiles) sans rien perdre d'irrécupérable. Utiliser quand l'utilisateur lance « Nettoyer le projet » depuis FabBoard, demande de faire le ménage, de supprimer les fichiers inutiles, de libérer de la place, ou lance /fab-cleanup. Inventaire, proposition classée, accord de l'utilisateur, puis mise à la Corbeille ou retrait par un commit. Pour Claude Code, Codex et Antigravity.
---

# Fab cleanup

Faire le ménage dans un projet, c'est supprimer : la seule tâche où une erreur ne se rattrape pas par un nouveau commit. Deux règles tiennent tout le reste.

1. **Rien n'est retiré sans l'accord de l'utilisateur**, donné sur une liste qu'il a vue.
2. **Rien n'est détruit** : un fichier suivi par Git se retire par un commit sur une branche (l'historique le garde) ; un fichier non suivi va à la **Corbeille** (`fabboard clean trash`), d'où il se reprend.

Ne jamais utiliser `rm`, `git clean`, `git reset --hard`, `git checkout -- <fichier>`, ni vider la Corbeille. Ne jamais réécrire l'historique. Ne rien toucher hors du projet.

## 1. Avant de commencer

1. `git status` : s'il y a des modifications en cours, le dire ; elles ne sont ni nettoyées ni embarquées dans un commit.
2. Travailler sur une branche dédiée (`fabboard/nettoyage`). Ne pas pousser.
3. Lire les consignes d'agent (`AGENTS.md`), le `README.md` et `.gitignore` : ils disent ce qui se reconstruit, comment, et ce qui est volontairement gardé hors de Git.
4. Noter l'état de départ : `fabboard status .` (tailles), et les commandes de test et de build du projet, qui serviront de preuve à la fin.

## 2. Inventaire

`fabboard clean scan` (ou `--json`) donne le point de départ : les dossiers reconstructibles et leur taille, les fichiers non suivis et ignorés, les plus gros fichiers suivis, les anciens builds de chaque version à tester, et ce que `clean trash` refusera (marqué « protégé »). Elle ne juge pas de l'utilité d'un fichier : compléter par un vrai regard sur l'arborescence.

Sa sortie est bornée : au-delà de quarante lignes par rubrique elle dit combien sont omises, et elle regroupe en une ligne les nombreux fichiers d'un même dossier. Un groupe n'est pas un dossier à retirer d'un bloc : ses fichiers se listent (`ls`) et se retirent chemin par chemin.

Ranger chaque candidat dans une de ces catégories.

| | Quoi | Exemples | Comment c'est retiré |
| --- | --- | --- | --- |
| **A. Reconstructible** | Ce qu'une commande refait | `target/`, `node_modules/`, `dist/`, `.venv/`, caches | Corbeille |
| **B. Résidus non suivis** | Ce qui traîne hors de Git et ne sert plus | fichiers temporaires, journaux, `.DS_Store`, captures d'essai, exports de test, anciens builds, sauvegardes (`*.bak`, `copie de …`) | Corbeille |
| **C. Suivis devenus inutiles** | Ce que Git suit et que plus rien n'emploie | documents périmés ou en double, notes de cadrage dépassées, code mort, fichiers jamais référencés, ressources inutilisées, scripts d'une étape révolue | `git rm` + commit |
| **D. À décider** | Ce dont l'utilité est incertaine | tout ce qui hésite entre B, C et « à garder » | Rien : on demande |

Pour la catégorie C, **chaque candidat vient avec sa preuve** : aucune référence dans le code, les scripts, la configuration de build, les tests ni la documentation (`grep -r` sur le nom du fichier et sur ce qu'il exporte), et, pour un document, ce qui l'a remplacé. « Il a l'air vieux » n'est pas une preuve.

### Ce qu'on ne propose jamais

- `.git/`, `roadmap/`, `ROADMAP.md`, les skills (`.agents/`, `.claude/`).
- L'histoire du projet : `DEVLOG`, ADR, rapports datés, `docs/ROADMAP-avant-fabboard.md`, notes de version.
- Ce qui sert de preuve ou d'entrée aux tests : corpus, jeux d'essai, fixtures, fiches de validation, y compris gros.
- Les données de l'utilisateur et ce qui ne se refait pas : `local-data/`, bases, fichiers de CAO, sources de logos et d'illustrations (`branding/`), licences.
- Les secrets et la configuration locale (`.env`, clés) : ne pas les lire, ne pas les déplacer.
- **Le build le plus récent** de chaque version à tester (`[[test]]` de `roadmap/projet.toml`, par exemple le dernier `.dmg` de `dist-desktop/`) : c'est ce que l'utilisateur essaie. Les plus anciens sont des candidats de la catégorie B.
- Les fichiers modifiés et non commités de l'utilisateur.

Dans le doute, catégorie D.

## 3. Proposer

Présenter la liste avant de toucher à quoi que ce soit, par catégorie : le chemin, la taille, pourquoi c'est inutile (la preuve pour C), et ce que coûte son retrait (« `target/` : 9,4 Go, une compilation complète de quelques minutes au prochain build »). Donner le total par catégorie et l'espace libéré.

Demander l'accord **par catégorie** pour A et B, **élément par élément** pour C, et une réponse pour chaque ligne de D. Un accord sur A ne vaut pas pour le reste.

Si l'utilisateur n'est pas là : ne rien retirer. Créer une action `decider` (« Choisir ce qui se nettoie dans <projet> ») avec la liste dans son corps, et s'arrêter.

## 4. Retirer ce qui est accordé

- **A et B** : `fabboard clean trash <chemin>…` (`--dry-run` pour chiffrer sans rien déplacer). Elle dit la place libérée. Elle valide tous les chemins avant d'en déplacer un : **un seul refus et rien ne bouge**. Elle refuse :
  - ce qui est hors du projet, ou la racine du projet ;
  - `.git/`, `roadmap/`, `ROADMAP.md`, `.agents/`, `.claude/` ;
  - un fichier suivi par Git, ou un dossier qui en contient un ;
  - un secret (`.env`, clés), ou un dossier non reconstructible qui en contient un ;
  - le build le plus récent d'une version à tester, ou le dossier qui le contient ;
  - un dépôt Git imbriqué ;
  - tout, si le projet n'est pas un dépôt Git : sans Git, on ne sait pas ce qui est suivi.

  Un refus n'est pas un obstacle à contourner : retirer le chemin refusé de la liste, relancer, et dire à l'utilisateur ce qui a été refusé et pourquoi.
- **C** : `git rm <chemins>`, puis un commit par ensemble cohérent, dont le message dit quoi et pourquoi (« Retire les notes de cadrage remplacées par docs/ARCHITECTURE.md »). Corriger dans le même commit les liens et les mentions qui pointaient vers ces fichiers.
- Pour un résidu qui reviendra (journaux, exports d'essai), proposer la ligne à ajouter à `.gitignore`.

Puis **prouver que rien n'est cassé** : relancer les tests et le build du projet. Après un retrait de la catégorie A, le premier build est long : le dire avant de le lancer, et demander s'il faut le faire maintenant. Si un test échoue à cause d'un retrait de la catégorie C : `git revert` du commit en cause, et le fichier repasse en catégorie D.

## 5. Bilan

1. `fabboard status .` : la taille avant et après.
2. Dire à l'utilisateur ce qui a été retiré par catégorie, ce qui a été gardé et pourquoi, ce qui attend sa décision, que les fichiers de la Corbeille s'y reprennent tant qu'elle n'est pas vidée, et la branche à fusionner. Ne pas fusionner ni pousser.
3. S'il reste des décisions : une action `decider` qui les liste. Sinon, si des fichiers suivis ont été retirés : une action `valider` (« Valider le nettoyage de <projet> ») qui dit la branche et les commits.
