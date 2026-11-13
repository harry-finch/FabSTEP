# Revue : anti-modèles et liste de contrôle

À passer en plus de la revue de `fab-design-system`, pour tout écran qui porte une illustration.

## Anti-modèles (refuser)

| Anti-modèle | À la place |
| --- | --- |
| Illustration d'une autre famille : image de banque, personnage, scène, pictogramme agrandi, dessin généré, émoji | Une figure Hairline, ou rien |
| Figure dessinée à la main, en SVG libre ou en retouchant `kernel.js` / `bench.html` | `hairline-create` ou `hairline-adopt`, moteur intact |
| Figure dans un panneau, une barre, un menu, une ligne de liste, une fenêtre de saisie | Phrase en muted, icône Lucide de 24 px au plus ; seule exception, l'aperçu paramétrique d'une fiche |
| Aperçu paramétrique sans mention « proportions », ou qui tient lieu de schéma coté | Une ligne en muted ; le schéma à l'échelle reste au canevas |
| Cote manquante dessinée comme si elle était connue | Valeur vraisemblable en pointillé, et la ligne en muted le dit |
| Figure à côté ou derrière le travail de l'utilisateur | La retirer dès qu'il y a quelque chose à montrer |
| Deux figures visibles à la fois ; une figure par étape d'un assistant | Une seule, à l'accueil |
| Figure comme indicateur d'attente ou de progression | État réel en texte (`TaskList`, barre d'état) |
| Figure qui bouge seule dans l'application (`play`, `phosphor`, `slow`) | Repos ; `play` au premier lancement et hors application seulement |
| Couleur sur une figure : accent, couleur de sens, `--identity`, valeur en dur, option `theme` | Jetons `--hairline-*` de `app.css`, rien d'autre |
| Plaques visibles (taches plus claires ou plus sombres que le fond) | `surface` égale à la surface réelle |
| Cadre, carte, ombre, fond teinté autour de la figure | La figure nue sur sa surface |
| Figure sans phrase, ou dont dépend la compréhension de l'état | La phrase dit tout ; la figure accompagne |
| Texte, chiffre, flèche ou logo dans le dessin ; légende de la figure (`rest`, `03`) affichée | Géométrie seule ; textes dans `fr.ts` / `en.ts` |
| `label` absent, en dur, ou laissé à la description anglaise du paquet | `t('figure.<nom>')` dans les deux langues |
| Figure de moins de 160 px ou de plus de 320 px dans l'application, étirée ou rognée | 240 px, rapport 5:4 |
| Figure détournée de son sens (un cadenas pour « enregistré ») ; deux figures pour le même état selon l'application | Tableau de la famille ([catalogue.md](catalogue.md)) |
| Concept choisi sans l'utilisateur ; figure livrée sans planche lue | Deux ou trois concepts proposés ; `look.mjs` à 0 et planche lue |
| `ui/public/illustrations/<name>.js` retouché, ou différent de sa source | Corriger `branding/illustrations/<name>.js`, recopier |
| Logo d'un tiers adopté sans accord | Demander à l'utilisateur |
| Paquet chargé depuis un CDN | Dépendance npm, application hors ligne |

## Liste de contrôle

**Installation**

- [ ] `@lucasmarkes/hairline` dans `ui/package.json` ; `Figure.svelte` et `figures.ts` en place ; jetons `--hairline-*` dans `app.css`.
- [ ] Pour une figure maison : skills `hairline-create` et `hairline-adopt` dans `.agents/skills/`, `skills-lock.json` versionné.
- [ ] `npm run check` : 0 erreur, 0 avertissement ; `npm run build` passe.

**Emplacement et composition**

- [ ] La figure est à l'un des six emplacements ; elle disparaît dès que le travail existe.
- [ ] Une seule figure visible ; figure, phrase, un bouton secondaire au plus.
- [ ] La figure est celle de la famille pour cet état.
- [ ] Largeur de 160 à 320 px, rapport 5:4, rien autour.

**Style**

- [ ] Aucune couleur réglée sur la figure ni autour (`grep -rn -e '--hairline-' ui/src` ne sort que `app.css` et `Figure.svelte`).
- [ ] `surface` nomme la surface réelle : aucune plaque ne se détache du fond, en sombre **et** en clair.
- [ ] Un seul endroit vif au repos ; il passe à ce que le pointeur choisit, puis revient.

**Comportement**

- [ ] Au repos sans pointeur ; répond au pointeur ; revient au repos quand il part.
- [ ] `play` absent, sauf premier lancement et hors application.
- [ ] Avec `prefers-reduced-motion`, rien ne bouge seul et la figure répond encore.
- [ ] La figure ne retarde ni ne masque aucune action : l'écran s'emploie sans elle.

**Textes et accessibilité**

- [ ] `figure.<nom>` présent dans `fr.ts` et `en.ts`, phrase courte qui décrit l'objet.
- [ ] La phrase d'accompagnement dit ce qui manque et quoi faire ; elle se suffit.
- [ ] Aucun texte dans le dessin ; la légende de la figure n'est pas affichée.

**Figure maison**

- [ ] Concept choisi par l'utilisateur parmi ceux que la skill a proposés.
- [ ] `look.mjs` sort 0 ; la planche a été lue et les questions de `look.md` ont leur réponse ; ce qui n'a pas pu être vérifié est dit.
- [ ] `diff branding/illustrations/<name>.js ui/public/illustrations/<name>.js` et `diff .agents/skills/hairline-create/kernel.js ui/public/illustrations/kernel.js` ne sortent rien.
- [ ] `branding/illustrations/README.md` note le concept, la date, l'emplacement.

**Aperçu paramétrique**

- [ ] En tête de la fiche (création et consultation), collant ; une ligne en muted dit « proportions ».
- [ ] Chaque champ de géométrie redessine la figure ; une cote vide est en pointillé ; les cas extrêmes restent dans le cadre.
- [ ] Sans paramètres (page d'essai, premier lancement), la figure est complète et répond au pointeur.

**Vérification**

- [ ] Vue dans le navigateur (ou l'application) : sombre et clair, 1440 et 1100 px de large, pointeur sur la figure puis parti.
