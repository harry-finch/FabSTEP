# Philosophie UX de la famille Fab

Ce qui fait qu'une application « ressemble à FabSpeed » tient moins aux couleurs qu'à une manière de se comporter. Ces principes priment sur toute règle de détail : en cas de doute, revenir ici.

## Pour qui

- **Makers, hobbyistes, fablabs.** Des gens compétents de leurs mains, pas forcément ingénieurs : ils connaissent leur machine et leurs matériaux, pas toujours le vocabulaire théorique (« charge de copeau », « taux d'ouverture »).
- Ils travaillent **sur un ordinateur d'atelier**, souvent à la souris, parfois sur un écran moyen (1280 px), parfois en anglais.
- Ils veulent **un résultat qui marche à la première découpe**, et comprendre pourquoi quand ce n'est pas le cas.

Conséquences : pas de jargon sans infobulle, des valeurs par défaut sûres, des exemples prêts à l'emploi (projet exemple, gabarits), une explication à portée de survol plutôt qu'une documentation à part.

## Les dix principes

### 1. Le travail de l'utilisateur est au centre

L'espace de travail (canevas, schéma de coupe, résultat) occupe le centre et le plus de place. Les panneaux sont étroits (≈ 300 px), sombres, calmes. Rien ne clignote, rien n'est décoratif. Une couleur vive sur l'écran doit **signifier** quelque chose.

### 2. Un logiciel professionnel, pas un tableau de bord

La référence visuelle est l'application de bureau créative (Illustrator, Lightroom, InDesign) : surfaces neutres, inspecteur contextuel, contrôles compacts de 28 px, petites capitales pour les sections. À l'inverse d'un SaaS : pas de grandes cartes décoratives ; des encarts compacts et utiles sont possibles, pas de dégradés, pas d'images d'ambiance, pas de gros boutons arrondis. Les seules illustrations sont des figures au trait, neutres, là où il n'y a pas encore de travail à montrer : état vide de l'espace de travail, premier lancement, À propos (skill `fab-illustration-system`).

### 3. Le moteur est honnête

- Une valeur saisie **n'est jamais corrigée en silence**. Hors domaine, elle produit un message à code stable (`plate.margin_too_large`), le champ concerné est souligné en rouge, et le message dit ce que le moteur en fait.
- Ce qui est retiré, plafonné ou adapté est **compté et signalé** (« 12 perforations retirées près du bord », « Avance plafonnée par la machine »).
- La gravité visuelle suit la gravité réelle : une adaptation attendue est une remarque grise repliée, pas un bandeau orange.

### 4. On sait d'où vient chaque chiffre

Quand une valeur peut avoir plusieurs origines (essai validé, donnée de référence, calcul, saisie de l'utilisateur, valeur dérivée), une petite étiquette icône + texte le dit à côté du chiffre (`Tag`, `OriginTag` de FabSpeed). La saisie de l'utilisateur prime toujours, et un bouton discret permet de revenir à la valeur proposée.

### 5. Le calcul est immédiat

Chaque modification relance le calcul : l'utilisateur voit le résultat changer sous ses yeux, sans bouton « Calculer ». Techniquement : un seul calcul en cours, seul le plus récent est envoyé (*latest wins*), le moteur garde en cache les étages inchangés. Si un calcul dépasse ≈ 60 ms pendant un geste (glisser une poignée, faire défiler un libellé), les suivants attendent le relâchement et la barre d'état le signale.

### 6. L'inspecteur suit le contexte

- Rien de sélectionné → réglages du document ou du canal affiché.
- Un objet sélectionné → ses propriétés, son nom, ses actions (dupliquer, supprimer).
- Les mesures et les messages restent visibles sous l'inspecteur, quel que soit le contexte.

On ne montre jamais tous les réglages à la fois.

### 7. Divulgation progressive

Ce qui sert souvent est visible. Le reste est derrière :

- une **section repliable** (état mémorisé par poste) ;
- une ligne **« ▸ Avancé »** (`Disclosure`) ;
- une **infobulle** (ⓘ à côté du libellé, ou au survol d'un bouton) ;
- l'**aide** (`?`), courte : quatre étapes et les raccourcis.

Une fenêtre modale est réservée à ce qui interrompt vraiment : aide, premier lancement, saisie d'un essai, validation.

### 8. Une seule action principale

Un seul bouton bleu (`btn primary`) par contexte : dans la barre d'application, l'action qui termine le travail (Exporter pour FabVents, Noter l'essai pour FabSpeed). Dans une fenêtre, le bouton de validation. Tout le reste est secondaire (`btn`) ou discret (`btn quiet`).

### 9. Rapide au clavier et à la souris

- Raccourcis d'une lettre pour les outils (V, H, P…), chiffres pour les onglets, `F` cadrer, `?` aide, `Échap` revenir, `⌘Z / ⇧⌘Z`, `⌘O / ⌘S`.
- Champs numériques : flèches ± pas (Maj × 10, Alt ÷ 10), glisser sur le libellé pour faire défiler la valeur, Entrée valide, Échap annule.
- Glisser-déposer un fichier n'importe où dans la fenêtre.
- Annuler / rétablir partout, les modifications successives d'un même champ fusionnées en une étape.

### 10. Rien ne se perd

Le document est sauvegardé localement 400 ms après chaque modification et repris au lancement. Les préférences d'affichage (langue, thème, unités, sections ouvertes) sont mémorisées par poste. Les fichiers exportés portent le nom du projet. Le format de fichier est versionné et les anciens fichiers se relisent.

## Ton et comportement

- **Sobre et précis** : « ✓ Disponible sur cette machine » plutôt que « La fraise sélectionnée est actuellement disponible sur la machine active ».
- **Bienveillant sans être bavard** : un état vide dit quoi faire (« Aucun essai pour ce contexte. Faites un premier passage sur une chute avec les valeurs proposées, puis notez le résultat. »).
- **Concret** : les infobulles parlent d'atelier (« la fraise traverse la plaque jusqu'au martyr »), pas de théorie.
- **Confirmations légères** : un *toast* de 2 à 3 s en bas (« Valeur copiée »), jamais une fenêtre « OK ».

## Relation avec `professional-workshop-ui`

`professional-workshop-ui` décrit le langage commun en termes généraux (densité, couleur sémantique, hiérarchie des boutons). Cette skill en est l'implémentation concrète et chiffrée pour les applications Fab : quand les deux divergent sur un détail, la valeur chiffrée d'ici l'emporte, car elle est vérifiée dans FabSpeed et FabVents.

## Assistance et visibilité du travail

Une tâche longue expose son état réel et ses étapes utiles ; une proposition indique ses sources et attend une décision explicite lorsqu’elle modifie le travail. Voir [beautiful-ui.md](beautiful-ui.md). Ces motifs complètent le travail central sans imposer une interface de chat.
