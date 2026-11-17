# Migrer une ancienne feuille de route

Pour un projet dont FabBoard signale `roadmap.legacy` : un `ROADMAP.md` écrit à la main, dans un format propre au projet. Le but est de retrouver le même contenu dans `roadmap/`, sans rien perdre et sans rien inventer.

## Avant de commencer

1. `git status` : travailler sur une branche dédiée (`fabboard/migration-roadmap`) si l'utilisateur ne dit pas autrement. Ne pas pousser.
2. Si `roadmap/` n'existe pas, s'arrêter : c'est FabBoard qui le crée (« Mettre d'équerre »).
3. Lire **tout** l'ancien `ROADMAP.md`, puis `STATUS.md`, `DEVLOG.md` et les documents qu'il cite, pour connaître l'état réel : la feuille de route peut être en retard sur le code.

Pendant le travail, `fabboard roadmap lint` et `fabboard doctor` signalent encore `roadmap.legacy` : c'est normal tant que l'ancien fichier n'est pas déplacé (dernière étape).

**Ne pas passer `--commit` aux commandes `fabboard` pendant une migration** : l'option commite sur-le-champ, une commande après l'autre. La migration se termine par un seul commit, fait à la main.

## Correspondances

| Dans l'ancien fichier | Devient |
| --- | --- |
| Introduction, but du projet, principes | `roadmap/intro.md`, repris tel quel, sans le titre du document ni les séparateurs (`ROADMAP.md` généré a sa propre mise en page). Les liens relatifs restent valides : `ROADMAP.md` reste à la racine |
| Versions ou jalons (« v0.7 », « M2b ») | `[[jalons]]` de `projet.toml`, dans l'ordre, avec leur titre. Ajouter sans rien retirer du fichier, commentaires compris |
| Un jalon ou un lot **terminé et validé** | Un élément `livree` par jalon ou par lot, corps = le texte d'origine recopié |
| Un jalon **développé mais pas encore validé** par l'utilisateur, ou validé en partie (une machine sur trois) | Élément `a-tester`, corps = le texte d'origine, et **une action par vérification qui reste** (`tester` ou `valider`) |
| Un travail **en cours**, y compris un jalon à moitié fait | Élément `en-cours`, corps = le texte d'origine. Le contrôle signalera une spécification incomplète : c'est exact, et c'est ce qui invitera l'utilisateur à la compléter avec l'agent. Ne pas la remplir de « À préciser. » |
| Une validation reçue **avec une réserve** (jalon validé, un critère resté ouvert) | L'élément est `livree` ; la réserve devient une action rattachée à cet élément |
| Une **suite annoncée dans une étape** (« lots 2 et 3 à venir ») | Un élément `idee` par suite, sans jalon si le document n'en donne pas ; le texte d'origine reste aussi dans l'élément qui l'annonçait |
| Un travail **à venir**, décrit avec but, contenu et critères | Élément `specifiee` : répartir le texte existant dans les six sections, sans le réécrire. S'il manque de la matière pour une section, l'élément reste `idee` avec le texte d'origine en corps |
| Un travail à venir en une ligne, une piste, une « suite différée » | Élément `idee` |
| Un point d'arrêt **en attente aujourd'hui** (« ⏸ validation », « à confirmer par l'utilisateur », « prochaine action : … ») | Une action du bon type, rattachée à son élément. Seulement ce que les documents demandent explicitement à l'utilisateur |
| Un point d'arrêt déjà franchi (« validé le … »), ou prévu dans un travail pas encore commencé | Reste dans le texte recopié (pour le second : sous « Vérifications de l'utilisateur ») ; pas d'action |
| Un travail **écarté**, ou « hors périmètre, à réévaluer plus tard » | Élément `abandonnee`, avec dans le corps le texte d'origine, la raison et l'échéance de réévaluation s'il y en a une. La décision est déjà prise dans le document : pas d'action |
| Un tableau de constats, de risques, de mesures | Reste dans le document d'origine (voir « L'ancien fichier ») et se cite depuis l'élément concerné |

Granularité : **un élément par chose qu'on peut livrer et vérifier séparément**. Un jalon déjà livré peut rester un seul élément ; un jalon à venir se découpe selon ses lots s'ils sont nommés dans le texte. Dans le doute, garder le découpage de l'auteur.

L'ordre de `ordre.txt` reprend l'ordre du document, sauf priorité explicite (« à la demande de l'utilisateur, X précède Y »).

## Dates

Les dates ne s'inventent pas ; elles se cherchent, dans cet ordre :

- `livree` : la date écrite dans la feuille de route ; sinon celle de l'entrée du `DEVLOG` ou du `STATUS` qui note la validation par l'utilisateur ; sinon celle du commit qui a marqué le jalon terminé (`git log -S "<texte>" -- ROADMAP.md`).
- `cree` : la date du commit qui a introduit le jalon dans la feuille de route ; à défaut, celle de sa livraison ; pour ce qui n'a jamais été daté, le jour de la migration. Que beaucoup d'éléments partagent la date du premier commit de la feuille de route est normal : ne pas chercher plus loin.

Elles se posent à la création : `fabboard roadmap add "Titre" --resume "…" --jalon v0.4 --cree 2026-10-30`, puis `fabboard roadmap set R-004 --statut livree --livree 2026-11-06`. Le corps se passe par `--corps-fichier chemin.md`, ou s'écrit ensuite dans le fichier.

## Règles

- **Ne pas inventer.** Pas de date, de critère ni de périmètre qui ne soit dans les documents du projet. Un statut vraiment incertain se décide avec l'utilisateur (action `decider`), pas au jugé.
- **Ne pas résumer** le texte des jalons livrés : il fait partie de l'histoire du projet. Le recopier. Seul `resume` (l'en-tête) est une phrase de ta main, fidèle au texte.
- **Corriger les liens relatifs** des textes recopiés : un lien `docs/FEATURES.md` écrit pour la racine devient `../../docs/FEATURES.md` dans `roadmap/items/`. `intro.md`, lui, est repris dans `ROADMAP.md` à la racine : ses liens ne changent pas.
- **Garder les identifiants du projet** (« v0.5 », « M2b », « AUD-017 », « CAT-01 ») dans les titres ou les corps : d'autres documents les citent.
- **Ne rien changer d'autre** : ni code, ni autre document, sauf les liens vers `ROADMAP.md` qui viseraient une ancre disparue. Si une autre skill ou un document du projet décrit encore `ROADMAP.md` comme écrit à la main, le signaler dans le rapport au lieu de le corriger.

## L'ancien fichier

Il n'est pas supprimé : `git mv ROADMAP.md docs/ROADMAP-avant-fabboard.md` (créer `docs/` au besoin), avec en tête une ligne qui dit qu'il est remplacé par `roadmap/` depuis telle date et conservé pour l'histoire. Corriger les liens relatifs qu'il contient si le déplacement les casse. Ensuite seulement : `fabboard roadmap render`, qui écrit le nouveau `ROADMAP.md`.

Le `ROADMAP.md` généré est un sommaire : titre, description rapide, statut et lien de chaque élément. Le détail des jalons est dans les fichiers de `roadmap/items/` et dans le document conservé.

Mettre à jour les consignes d'agent du projet (`AGENTS.md`) : là où elles parlent de `ROADMAP.md`, dire que la feuille de route est dans `roadmap/` et renvoyer à la skill `fab-roadmap`.

## Contrôle

1. `fabboard roadmap lint` : aucune erreur ; les avertissements restants sont expliqués dans le rapport.
2. **Rien de perdu** : relire l'ancien fichier section par section et pointer, pour chacune, où elle se trouve maintenant (élément, `intro.md`, document conservé). Donner ce tableau à l'utilisateur.
3. `fabboard show .` : l'avancement et le jalon en cours correspondent à ce que dit `STATUS.md`.
4. Lire le `ROADMAP.md` généré comme le lira quelqu'un sur GitHub.

## Livrer

1. Créer l'action finale : `fabboard action add --type valider "Valider la migration de la feuille de route" --corps "…"`, avec dans son corps le nombre d'éléments par statut, les choix faits là où le document était ambigu, les dates dont la source n'est pas la feuille de route, et ce qui demande son avis.
2. **Un seul commit** sur la branche, action comprise : `roadmap: migration de l'ancienne feuille de route`.
3. Ne pas fusionner ni pousser : c'est sa décision.
