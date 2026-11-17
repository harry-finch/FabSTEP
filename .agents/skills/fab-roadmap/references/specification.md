# Écrire une spécification

Une spécification est le contrat entre l'utilisateur et l'agent qui développera, peut-être dans une autre session, sans l'historique de la conversation. Elle doit suffire à développer et à vérifier. Elle se lit dans FabBoard : phrases courtes, listes, pas de roman.

À partir du statut `specifiee`, le corps du fichier contient ces six sections, **dans cet ordre, avec ces titres exacts** (niveau 2). D'autres sections peuvent suivre (notes, décisions, maquettes).

## Les six sections

### `## But`

Le besoin, en deux à quatre phrases : qui a quel problème, et ce qui change pour lui une fois l'élément livré. Pas la solution.

> Éviter les plaques fragiles : le moteur compare chaque nervure au minimum de la matière choisie et le dit, au lieu de laisser l'utilisateur le découvrir à la découpe.

### `## Inclus`

Ce qui sera fait, en liste. Chaque ligne est un comportement observable ou un livrable, pas une tâche technique.

- Bien : « Message à code stable, compté, avec la nervure la plus fine. »
- Mal : « Refactoriser le module des mesures. »

### `## Hors périmètre`

Ce qui ne sera **pas** fait, alors qu'on pourrait s'y attendre. C'est la section qui évite les malentendus à la livraison : y mettre tout ce qui a été écarté pendant la discussion, et les suites reportées (avec leur identifiant si elles deviennent des éléments).

### `## Dépendances`

Éléments requis avant (`R-…`), décisions à prendre, documents ou fichiers à fournir par l'utilisateur. « Aucune » si rien. Une dépendance envers l'utilisateur se double d'une action `fournir` ou `decider`.

### `## Critères d'acceptation`

Des faits vérifiables, par un test ou à l'œil, sans interprétation.

- Bien : « Une plaque de 3 mm d'acrylique signale toute nervure sous 1,0 mm. »
- Bien : « `cargo test --workspace` et `npm run check` sans erreur. »
- Mal : « L'interface est fluide. » → « Une grille de 2 000 trous se recalcule pendant le déplacement d'un contrôleur sans saccade visible. »

Inclure les cas limites et les refus attendus (« un nom vide est refusé, avec la raison »).

### `## Vérifications de l'utilisateur`

Ce que **lui seul** peut faire : essayer sur sa machine, fabriquer une pièce, juger d'un rendu, relire un texte. Une ligne par vérification, à l'infinitif ; chacune deviendra une action quand l'élément passera à `a-tester`. « Aucune » si tout se vérifie par les tests.

## Ce qui fait une bonne spécification

- **Elle tranche.** Chaque question ouverte pendant la discussion a sa réponse dans le texte, ou est listée en dépendance.
- **Elle est bornée.** Si « Inclus » dépasse une dizaine de lignes ou mêle plusieurs besoins, proposer de découper.
- **Elle ne décrit pas le code.** Le comment appartient à l'agent qui développe, sauf contrainte voulue par l'utilisateur (une bibliothèque, un format, une compatibilité) : celle-ci s'écrit dans « Inclus ».
- **Elle reprend les mots de l'utilisateur** pour les choses du métier, et les termes du projet pour le reste.
- **Elle est honnête sur l'incertain** : une hypothèse est écrite comme une hypothèse.

## Avant de demander la validation

- [ ] Les six sections sont là, dans l'ordre, avec leurs titres exacts.
- [ ] `resume` (dans l'en-tête) dit encore l'essentiel en une ou deux phrases.
- [ ] Chaque critère d'acceptation se vérifie sans demander « c'est-à-dire ? ».
- [ ] « Hors périmètre » contient ce qui a été écarté.
- [ ] `fabboard roadmap lint` ne signale rien sur ce fichier.
