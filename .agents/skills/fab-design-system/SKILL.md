---
name: fab-design-system
description: Thème, style graphique et philosophie UX communs aux applications Fab (FabSpeed, FabVents et les suivantes) pour makers et fablabs. Utiliser pour créer une nouvelle application de la famille, ajouter ou revoir un écran, un composant, un canevas, des textes ou un logo, ou vérifier qu'une interface « fait partie de l'écosystème ». Jetons CSS chiffrés (sombre et clair), composants Svelte 5 prêts à copier, règles de mise en page, d'interaction, de rédaction FR/EN et d'identité.
---

# Fab design system

Le système de design des applications Fab : une interface d'atelier **professionnelle, compacte, sombre et calme**, inspirée des applications créatives de bureau, pour des **makers et des fablabs**. FabSpeed (feeds & speeds) et FabVents (motifs de perforation) en sont les deux références : le kit existant en reprend les bases ; le kit comprend aussi des composants inspirés de Beautiful UI, à adapter aux besoins du projet.

Une application de la famille se reconnaît à :

- un **liseré de 2 px** à sa couleur d'identité en haut de la fenêtre, un logo « tuile graphite + symbole coloré », le nom « Fab… » en gras ;
- trois colonnes : **réglages à gauche** (sections numérotées en petites capitales), **le travail au centre**, **inspecteur et mesures à droite**, barre d'état de 24 px ;
- des contrôles de **28 px**, des libellés de 11,5 px en gris, des valeurs alignées à droite avec leur unité, des sélecteurs segmentés en relief ;
- un seul bouton bleu, des couleurs qui ne servent qu'à dire un état, des infobulles partout ;
- un moteur **honnête** (rien n'est corrigé en silence) et un résultat qui se met à jour **en direct**.

## Apport Beautiful UI

[Beautiful UI](https://www.beautifului.dev/) inspire des éléments compacts, explicites et contextuels : progression par étapes, tableaux filtrables, recherche, sources consultables et propositions à examiner. Employer ces motifs quand le besoin existe, avec les jetons et la densité Fab. Les encarts servent une décision ou un contexte précis ; ils ne transforment pas les panneaux en tableau de bord. Voir [beautiful-ui.md](references/beautiful-ui.md).

## Pile technique

Svelte 5 (runes) + Vite + TypeScript, icônes `@lucide/svelte`, police Inter Variable (`@fontsource-variable/inter`). Moteur en Rust (WebAssembly dans un Web Worker, ou service local). Bureau : Tauri. Les principes valent aussi pour une autre pile (Qt, egui) : reprendre alors les valeurs des jetons et des dimensions.

## Démarrer une nouvelle application

1. Copier [`assets/starter/`](assets/starter/) dans `ui/` du nouveau projet, puis `npm install`.
2. Remplacer les marqueurs : `__app__` (identifiant en minuscules : clés de stockage, `package.json`, `index.html`) et `__App__` (nom affiché) dans `src/lib/app.ts`, `index.html`, `package.json`, `public/logo.svg`.
3. Choisir la couleur d'identité (règles dans [identite-et-logo.md](references/identite-et-logo.md)) et la mettre dans `--identity` des deux thèmes de `src/app.css` et dans `fill` du symbole de `public/logo.svg`.
4. Écrire le sous-titre (`app.tagline`) dans `src/lib/locales/fr.ts` et `en.ts`.
5. Remplacer le contenu d'exemple de `src/App.svelte` (il montre chaque composant) par les panneaux de l'application ; garder la grille, la barre d'application, la barre d'état et les toasts.
6. Ajouter les jetons du domaine (canevas, schémas) dans le bloc prévu de `app.css`, **dans les deux thèmes**.
7. `npm run check` (0 erreur, 0 avertissement) et `npm run dev`, vérifier en thème sombre et clair.

Le kit contient : `app.css` (jetons et styles de base, dont les jetons des illustrations), `lib/` (`app.ts`, `ui.svelte.ts` unités / aide / toasts, `theme.svelte.ts`, `i18n.svelte.ts` + `locales/`, `format.ts`, `tip.ts`) et `components/` (`AppBar`, `StatusBar`, `Section`, `Disclosure`, `Field`, `NumField`, `Segmented`, `Popover`, `Dialog`, `Alerts`, `Tag`, `TaskList`, `ContextCard`, `ProposalCard`, `RecordsTable`, `AssistantPanel`). Il compile tel quel (vérifié avec svelte-check et vite build).

## Règles non négociables

1. **Aucune couleur en dur** dans un composant ou dans le code de dessin : jetons de `app.css` uniquement, définis pour le thème sombre (référence) et le thème clair.
2. **Couleur = sens.** Bleu : action, sélection, focus. Vert : validé. Orange : attention. Rouge : bloquant. Cyan : information. L'identité : logo et liseré seulement. Jamais la couleur seule : toujours une icône ou un mot.
3. **Densité d'atelier** : contrôles de 28 px, rayon 4 px, rythme 4 / 8 / 12 / 16 px, textes de 10,5 à 14 px. Pas de grandes cartes décoratives, pas d'ombres sur les panneaux, pas de dégradés ; des encarts compacts fonctionnels sont permis. Pas d'image d'ambiance : les seules illustrations sont les figures au trait de la skill `fab-illustration-system`, à leurs emplacements.
4. **Une propriété par ligne** : `LIBELLÉ (100 px) | VALEUR (à droite) | UNITÉ (30 px)`. Sélecteur de 3 options ou plus : sous-libellé puis pleine largeur.
5. **Une seule action principale** (`btn primary`) par contexte.
6. **Le moteur ne corrige rien en silence** : message à code stable, compté, rendu par l'interface dans la langue ; champ fautif souligné ; gravité visuelle = gravité réelle.
7. **En direct et réversible** : pas de bouton « Calculer » ; tout est annulable, un geste = une étape ; le document et les préférences sont mémorisés.
8. **Inspecteur contextuel et divulgation progressive** : on montre ce qui sert maintenant, le reste derrière une section, « ▸ Avancé » ou une infobulle.
9. **Textes** : français de référence, anglais complet et vérifié par TypeScript, typographie française, ton sobre et concret, infobulle sur chaque terme technique.
10. **Millimètres et degrés** dans le moteur et les fichiers ; les pouces ne sont qu'un affichage.

## Références

Lire selon la tâche :

| Fichier | Contenu | Quand |
| --- | --- | --- |
| [philosophie.md](references/philosophie.md) | Public, dix principes UX, ton | Toujours, au moins une fois |
| [jetons.md](references/jetons.md) | Toutes les valeurs (sombre / clair), rôles, jetons du domaine | Style, nouveau jeton, thème clair |
| [typographie.md](references/typographie.md) | Échelle, chiffres, petites capitales, longueurs | Tout texte affiché |
| [mise-en-page.md](references/mise-en-page.md) | Anatomie, dimensions, panneaux, lignes de propriété, écrans moyens, z-index | Nouvel écran, panneau |
| [composants.md](references/composants.md) | Spécification de chaque composant (boutons, champs, segmentés, menus, fenêtres, messages, étiquettes, barre d'outils, onglets, listes, mesures, toasts, infobulles…) | Ajout ou revue d'un composant |
| [interactions.md](references/interactions.md) | Raccourcis, champs numériques, annuler, calcul en direct, persistance, fichiers, sélection, aide | Comportement |
| [textes-et-langues.md](references/textes-et-langues.md) | i18n, clés, messages du moteur, typographie FR, unités, style | Tout texte |
| [canevas.md](references/canevas.md) | Dessin par jetons, hiérarchie visuelle, poignées, étiquettes, schémas, export d'image | Espace de travail graphique |
| [identite-et-logo.md](references/identite-et-logo.md) | Couleurs de la famille, choix d'une identité, tuile, symbole, fichiers | Nouvelle application, logo |
| [beautiful-ui.md](references/beautiful-ui.md) | Motifs Beautiful UI adaptés : tâches, recherche, tableaux, sources, propositions et assistance | Tâches asynchrones, données ou assistance |
| [revue.md](references/revue.md) | Anti-modèles et liste de contrôle | Avant de livrer, revue de code UI |

## Façon de travailler

- **Avant d'ajouter un style** : chercher le jeton, la classe (`.btn`, `.input`, `.tabular`) ou le composant existant ; l'étendre plutôt que dupliquer.
- **Nouveau composant** : partir du plus proche dans [composants.md](references/composants.md), reprendre ses dimensions et ses états (survol, actif, focus, désactivé, erreur), et le documenter dans un commentaire d'en-tête en français (rôle et comportements), comme les composants du kit.
- **Nouvelle décision de style** (une valeur qui manque ici) : la prendre dans FabSpeed ou FabVents si elle y existe ; sinon la proposer à l'utilisateur, puis l'ajouter à cette skill pour que toute la famille en profite.
- **Vérifier dans le navigateur**, en sombre et en clair, à 1440 et à 1100 px de large, avant de déclarer un écran terminé ; passer la [liste de contrôle](references/revue.md).
- **Illustrations** (état vide de l'espace de travail, premier lancement, À propos, site, README) : skill `fab-illustration-system`. Elle fixe les emplacements, installe les figures au trait Hairline dans le projet et fournit le composant `Figure` ; leurs jetons `--hairline-*` sont déjà dans `app.css`. Aucune autre illustration.
- Pour les règles propres à un métier (sécurité machine, FAO, validation de géométrie), combiner avec la skill du domaine (`cam-ux-ui`, etc.). `professional-workshop-ui` donne les mêmes principes en termes généraux ; ici les valeurs chiffrées font foi.
