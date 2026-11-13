# Jetons de design

Source de vérité : [`assets/starter/src/app.css`](../assets/starter/src/app.css), identique (hors identité et jetons du domaine) à `FabSpeed/ui/src/app.css` et `FabVents/ui/src/app.css`. Toute couleur, taille ou ombre d'un composant **passe par un jeton**. Aucune valeur hexadécimale dans un composant ni dans le code de dessin d'un canevas.

## Surfaces (du plus sombre au plus clair)

| Jeton | Sombre | Clair | Rôle |
| --- | --- | --- | --- |
| `--bg-chrome` | `#181818` | `#e6e6e6` | Barre d'application, barre d'état, fond du `body` |
| `--bg-workspace` | `#1c1c1c` | `#f0f0f0` | Espace de travail central |
| `--bg-panel` | `#232323` | `#f8f8f8` | Panneaux latéraux, barre d'outils du canevas, encart du document |
| `--bg-elevated` | `#2b2b2b` | `#ffffff` | Menus, fenêtres, bouton secondaire, segment actif |
| `--bg-control` | `#161616` | `#ffffff` | Fond des champs de saisie (plus sombre que le panneau en thème sombre : le champ « creuse ») |
| `--bg-hover` | `#2f2f2f` | `#e3e3e3` | Survol d'une ligne, d'un bouton discret |
| `--bg-pressed` | `#363636` | `#d6d6d6` | Bouton enfoncé |
| `--bg-selected` | bleu 16 % | bleu 12 % | Ligne ou outil sélectionné (teinte, jamais un aplat bleu) |

Ordre de profondeur à respecter : chrome < workspace < panel < elevated. Un menu ou une fenêtre est toujours plus clair que ce qu'il recouvre (thème sombre) ou blanc (thème clair).

## Bords et séparateurs

| Jeton | Sombre | Clair | Rôle |
| --- | --- | --- | --- |
| `--border` | `#343434` | `#d0d0d0` | Bord des champs, des panneaux, des barres |
| `--border-strong` | `#444` | `#b3b3b3` | Bouton secondaire, survol d'un champ, séparateurs verticaux de barre d'outils, `kbd` |
| `--divider` | `#2c2c2c` | `#dedede` | Séparation entre sections, dans les menus, entre cellules |

## Texte

| Jeton | Sombre | Clair | Rôle |
| --- | --- | --- | --- |
| `--text-primary` | `#e8e8e8` | `#1e1e1e` | Valeurs, noms, titres |
| `--text-secondary` | `#ababab` | `#474747` | Libellés de champ, texte courant des panneaux |
| `--text-muted` | `#7d7d7d` | `#636363` | Unités, métadonnées, raccourcis, barre d'état, placeholder |
| `--text-section` | `#9a9a9a` | `#595959` | Titres de section en petites capitales |
| `--text-disabled` | `#555` | `#a3a3a3` | Élément de menu désactivé |
| `--on-accent` | `#fff` | `#fff` | Texte sur fond `--accent` |

Jamais de blanc pur ni de noir pur pour le texte.

## Couleurs de sens

| Jeton | Sombre | Clair | Sens |
| --- | --- | --- | --- |
| `--accent` | `#4b9cf5` | `#1473e6` | Action principale, focus, sélection, onglet actif |
| `--accent-hover` | `#63abf7` | `#0d66d0` | Survol du bouton principal |
| `--accent-text` | `#8cc0ff` | `#0a5bc2` | Texte ou icône en bleu (outil actif, lien) |
| `--success` | `#3fb67a` | `#0e7350` | Validé, prêt, moteur en marche |
| `--warning` | `#e8a33c` | `#975200` | Avertissement, adaptation, calcul différé |
| `--danger` | `#ef5b5b` | `#bb1f28` | Erreur bloquante, champ invalide |
| `--danger-text` | `#ffb4b4` | `#a1141b` | Texte d'une erreur sur fond `--danger-tint` |
| `--info` | `#4fc3d9` | `#096a7e` | Information, donnée de référence |

Teintes (fonds très légers, toujours accompagnés d'une icône ou d'un texte de la couleur pleine) : `--accent-tint`, `--success-tint`, `--success-border`, `--warning-tint`, `--danger-tint`, `--info-tint`, `--neutral-tint` (blanc 5 % / noir 5 %, pour les pastilles neutres et compteurs).

Règles :

- Le bleu ne veut dire que « action / sélection / focus ». Le vert, « bon / validé ». L'orange, « attention ». Le rouge, « bloquant ». Le cyan, « information ».
- Une couleur de sens s'emploie **en petit** : icône, texte court, liseré, pastille. Le seul fond teinté autorisé dans un panneau est celui d'une erreur bloquante (`--danger-tint`) ou d'une étiquette (`Tag`).
- Jamais la couleur seule : toujours une icône (`CircleX`, `TriangleAlert`, `Info`, `Check`) ou un mot.

## Identité

`--identity` : couleur propre à l'application. **Usages autorisés : le symbole du logo et le liseré de 2 px en haut de la barre d'application. Rien d'autre.** Elle ne désigne jamais un état, une sélection ni une action. Voir [identite-et-logo.md](identite-et-logo.md) pour les valeurs de la famille et le choix d'une nouvelle couleur.

## Rails

`--segment-track` et `--track` (`#161616` / `#e3e3e3`, `#e0e0e0`) : fond creusé d'un sélecteur segmenté ou d'une jauge.

## Fenêtres, infobulles, ombres

| Jeton | Sombre | Clair |
| --- | --- | --- |
| `--backdrop` | noir 50 % | noir 28 % |
| `--tooltip-bg` / `--tooltip-text` | `#383838` / `#ececec` | `#2b2b2b` / `#f2f2f2` (l'infobulle reste sombre en thème clair) |
| `--tooltip-shadow` | `0 6px 20px rgb(0 0 0 / .45)` | `… / .25` |
| `--shadow-pop` | `0 8px 28px rgb(0 0 0 / .5), 0 0 0 1px rgb(255 255 255 / .05)` | `0 8px 28px rgb(0 0 0 / .16), 0 0 0 1px rgb(0 0 0 / .08)` |
| `--scrollbar` | `#3a3a3a` | `#c2c2c2` |
| `--swatch-edge` | noir 25 % | noir 18 % (contour d'une pastille de couleur de matière) |

Exceptions tolérées, parce que ce sont des **données** et non de l'interface : les pastilles de couleur des matières (`FabSpeed/ui/src/lib/swatches.ts`) et la couleur fixe de la pièce dans une image exportée (voir [canevas.md](canevas.md)). Elles vivent dans un module de données, jamais dans un composant.

Une seule ombre : `--shadow-pop`, pour ce qui flotte (menus, fenêtres, toasts). Les panneaux n'ont pas d'ombre : ils sont séparés par un bord de 1 px.

## Rythme et formes

| Jeton | Valeur | Usage |
| --- | --- | --- |
| `--s1` | 4 px | Écart entre une icône et son texte, entre deux puces |
| `--s2` | 8 px | Écart entre lignes de propriétés, marge interne d'un champ |
| `--s3` | 12 px | Marge latérale des panneaux et des sections |
| `--s4` | 16 px | Marge des fenêtres, écart entre groupes de la barre d'application |
| `--s5` | 24 px | Grands écarts (rare) |
| `--control-h` | 28 px | Hauteur de tout contrôle (champ, bouton, segmenté, encart) |
| `--radius` | 4 px | Champs, boutons, lignes de menu |
| `--radius-lg` | 6 px | Menus, fenêtres, toasts |

Valeurs intermédiaires tolérées et déjà utilisées : 2 px (padding d'un rail, écart entre outils), 3 px (rayon d'un segment, d'une étiquette), 5–6 px (padding vertical des lignes de menu et d'alerte). Ne pas en inventer d'autres.

## Typographie

`--font` (Inter Variable, chargée par `@fontsource-variable/inter`), `--mono` (rare : code, identifiants), `--fs-xs` 10,5 px, `--fs-sm` 11,5 px, `--fs-md` 12,5 px (texte par défaut), `--fs-lg` 14 px. Détail dans [typographie.md](typographie.md).

## Illustrations

`--hairline-plate` (= `--bg-workspace`, réglé ensuite par surface) et `--hairline-hi` (= `--text-primary`) colorent les figures au trait de la skill `fab-illustration-system`. Les trois gris du trait (`--hairline-edge`, `--hairline-mid`, `--hairline-lo`) en sont tirés par `color-mix` (40 %, 26 %, 15 %) dans la règle `[data-hairline], .linework` de `app.css` : la figure garde sa hiérarchie sur toute surface, et un schéma au trait (classe `.linework`) prend les mêmes gris. Aucune valeur propre, donc rien à redéfinir dans le thème clair. Une figure ne prend ni l'accent, ni une couleur de sens, ni l'identité.

Ces jetons sont plus récents que FabSpeed et FabVents : les ajouter à une application existante quand elle reçoit sa première figure.

## Jetons du domaine

Chaque application ajoute ses propres jetons dans un bloc commenté de `app.css`, **dans les deux thèmes**, préfixés par leur rôle :

- FabVents, canevas : `--canvas-bg`, `--plate-fill` (`#4b4f55` / `#b9bdc3`), `--plate-edge`, `--plate-margin`, `--hole-fill` (= fond, le trou laisse voir le fond), `--ctrl`, `--ctrl-edge`, `--ctrl-reach`, `--ctrl-reach-fill`, `--ctrl-dim`, `--ctrl-active` (= accent), `--ctrl-active-fill`, `--canvas-ghost`.
- FabSpeed, origine des valeurs : `--origin-lab` (cyan), `--origin-calc` (gris), `--origin-user` (violet `#b58cff`), `--origin-derived` (orange), et leurs teintes ; jauges : `--gauge-marker`, `--gauge-tick`, `--meter-neutral` ; schéma de coupe : `--scene-material-tint` (opacité de la matière) et `--scene-danger` ; son trait vient des jetons `--hairline-*` par la classe `.linework` (voir [canevas.md](canevas.md)).
- FabWeave, vue 3D du parcours : préfixe `--view-*` (fond, plateau, grille, zones exclues : `--view-bg`, `--view-bed`, `--view-bed-edge`, `--view-grid`, `--view-bed-exclude`), à côté de `--path-*` (parcours) et `--scale-*` (rampe de coloration) ; la buse simulée (VIEW-08) a `--view-nozzle` (`#b9bdc3` / `#6b7078`, remplissage translucide de la buse) et `--view-nozzle-edge` (`#2a2d31` / `#2a2d31`, ses arêtes, identique dans les deux thèmes). L'outil reste neutre : aucune couleur de sens. Les valeurs sont lues par `Viewport.svelte` et relues au changement de thème (`FabWeave/ui/src/app.css`).

Règles pour un nouveau jeton du domaine :

1. Réutiliser d'abord un jeton commun (un fond de trou = `--canvas-bg`, une sélection = `--accent`).
2. Les objets de l'utilisateur sont **neutres** (gris, blanc cassé) ; seule la sélection ou l'objet actif prend l'accent.
3. Un jeton du domaine qui exprime un état reprend la couleur de sens correspondante (`--scene-danger` = rouge 35 %).
4. Vérifier le rendu dans les deux thèmes avant de valider.
