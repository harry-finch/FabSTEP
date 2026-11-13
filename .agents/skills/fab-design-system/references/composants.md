# Catalogue des composants

Chaque composant existe dans FabSpeed et/ou FabVents ; la colonne « Kit » indique s'il est fourni prêt à l'emploi dans `assets/starter/src/`. Icônes : `@lucide/svelte`, tailles indiquées.

| Composant | Kit | Rôle |
| --- | --- | --- |
| Boutons `.btn` (primaire, secondaire, discret, icône) | `app.css` | Actions |
| Champ `.input` | `app.css` | Saisie texte |
| `NumField` | ✓ | Propriété numérique avec unité |
| `Field` | ✓ | Propriété non numérique |
| `Segmented` | ✓ | Choix exclusif court |
| `Section` | ✓ | Section repliable de panneau |
| `Disclosure` | ✓ | « ▸ Avancé » |
| `Popover` | ✓ | Menu déroulant, sélecteur riche |
| `Dialog` | ✓ | Fenêtre modale |
| `Alerts` | ✓ | Messages du moteur |
| `Tag` | ✓ | Étiquette d'état ou d'origine |
| `AppBar` | ✓ (gabarit) | Barre d'application |
| `StatusBar` | ✓ (gabarit) | Barre d'état |
| `tip` (action) | ✓ | Infobulle |
| `Figure` | skill `fab-illustration-system` | Illustration au trait : état vide de l'espace de travail, premier lancement, À propos, aperçu paramétrique d'une fiche |
| Toasts | ✓ (`ui.toast`, `App.svelte`) | Confirmation passagère |
| Barre d'outils, onglets, curseur, barre de lecture, menu contextuel, sélecteur à pictogrammes, liste d'éléments, mesures, cellule de résultat, `kbd`, voile de dépôt | modèles ci-dessous | — |

## Boutons

Trois niveaux, jamais plus :

| Classe | Dessin | Usage |
| --- | --- | --- |
| `.btn.primary` | fond `--accent`, texte `--on-accent`, survol `--accent-hover` | **Une** action principale par contexte |
| `.btn` | fond `--bg-elevated`, bord `--border-strong`, survol `--bg-hover`, enfoncé `--bg-pressed` | Actions courantes (Ajouter, Importer…) |
| `.btn.quiet` | transparent, texte secondary, survol `--bg-hover` + primary | Actions de faible poids, icônes de barre |
| `.btn.icon` | carré de 28 px, sans padding ; à combiner avec `quiet` | Fermer, aide, dupliquer, supprimer |

Communs : hauteur 28 px, padding 0 12 px (quiet : 0 8 px), rayon 4 px, `--fs-sm` 500, icône 14 px + texte avec 6 px d'écart, `white-space: nowrap`, désactivé à opacité 0,45. Variante compacte dans une liste : 24 × 24 px.

Icône seule ⇒ `aria-label` **et** `use:tip` (avec le raccourci entre parenthèses : « Aide (?) »).

## Champ de saisie `.input`

Hauteur 28 px, fond `--bg-control`, bord `--border` (survol `--border-strong`, focus `--accent`, sans halo), rayon 4 px, padding 0 8 px. Erreur : bord `--danger` (classe `bad`) + `aria-invalid`. Placeholder en muted.

## `NumField` — propriété numérique

Grille `100px | 1fr | 30px` : libellé (secondary, `--fs-sm`, tronqué), champ aligné à droite en chiffres tabulaires, unité muted.

Comportements (tous obligatoires) :

- saisie dans la langue (virgule en français, espaces ignorés), validée à la sortie ou sur Entrée ; Échap rétablit la valeur ; une saisie illisible rétablit la valeur sans message ;
- la valeur entière est sélectionnée au focus ;
- ↑ / ↓ : ± pas (Maj × 10, Alt ÷ 10) ;
- **glisser horizontalement sur le libellé** (curseur `ew-resize`) : 1 pas tous les 4 px, une seule étape d'annulation (`onbegin` / `onend`) ;
- ⓘ (icône `Info` 11 px, muted) à côté du libellé quand un `hint` existe ;
- longueurs reçues et rendues en **mm**, affichées en mm ou en pouces selon `ui.units` ;
- `min` / `max` bornent la saisie au domaine physique (une largeur ≥ 0) ; les limites métier sont signalées par le moteur, jamais bridées ici ;
- `invalid` : le moteur signale une erreur sur ce champ (souligné rouge).

Facultatif, **retour à la valeur proposée** (FabWeave, même geste que le ↺ de la cellule de résultat de FabSpeed) : quand la valeur diffère de la valeur proposée par le modèle ou le moteur (`modified`) et qu'`onreset` est fourni, un ↺ (`RotateCcw` 11 px, muted, survol primary) suit le libellé et ⓘ. Son infobulle et son `aria-label` (`resetHint`) donnent la valeur proposée (« Valeur du modèle : 60 mm ») ; il arrête `pointerdown` pour qu'un clic ne lance pas le défilement du libellé. Avec ⓘ et ↺ dans la colonne, le libellé tient en 14 caractères (voir [typographie.md](typographie.md)).

## `Field` — propriété non numérique

Même colonne de libellé (100 px) ; contrôle dans `1fr` : `Segmented` à deux options, sélecteur, case à cocher, bouton. Case à cocher : `accent-color: var(--accent)`, texte `--fs-sm` secondary à droite.

## `Segmented` — choix exclusif

Rail 28 px (`--segment-track`, bord `--border`, padding 2 px, gap 2 px) ; segments égaux (`flex: 1`), rayon 3 px, `--fs-sm` 500 secondary ; segment actif : fond `--bg-elevated`, texte primary, `box-shadow: inset 0 0 0 1px var(--border-strong)` (relief, **pas de bleu**). Rôles `radiogroup` / `radio`, ← → pour changer, une infobulle par option (`hint`).

2 options → dans un `Field`. 3 ou 4 options → sous-libellé + pleine largeur. 5 options ou plus, ou options qui ont besoin d'un pictogramme et d'une description → sélecteur `Popover`.

Variante de la barre d'application (unités, langue, thème) : rail de 24 px, segments sans `flex: 1`, segment actif ombré par `--border`, icônes 12 px pour le thème (Monitor, Sun, Moon).

## `Section`, `Disclosure`

Voir [mise-en-page.md](mise-en-page.md). `Disclosure` : bouton 24 px, chevron 13 px, texte `--fs-sm` muted (survol primary), corps en colonne gap 8 px ; fermé par défaut, état mémorisé.

## `Popover` — menus et sélecteurs

- Panneau fixe (n'est pas rogné par un panneau qui défile), fond `--bg-elevated`, rayon 6 px, `--shadow-pop`, padding 4 px, largeur fixée (260–320 px), hauteur ≤ 520 px.
- S'ouvre sous le déclencheur, au-dessus s'il manque de place ; se ferme au clic extérieur, à Échap (focus rendu au déclencheur), au défilement d'un autre élément, au redimensionnement.
- Déclencheur : bouton avec chevron `ChevronDown` 13 px ; ouvert = fond `--bg-hover` (bouton de barre) ou bord `--accent` (sélecteur dans un panneau).
- **Ligne de menu** : `display: flex; gap: 8px; padding: 6px 8px; rayon 4 px`, icône 14 px secondary, nom primary 500, à droite raccourci (`--fs-xs` muted) ; survol `--bg-hover` ; désactivée à opacité 0,45.
- **Ligne riche** (sélecteur) : pictogramme 22 px, nom 500 + description `--fs-sm` muted sur deux lignes, coche à droite pour l'option courante, ligne courante sur `--bg-selected`.
- Séparateur : 1 px `--divider`, marge 4 px. Sous-titre de groupe : `--fs-xs` 600 majuscules 0,04em `--text-section`, marge `8px 8px 4px`.
- Pastilles d'action rapide (gabarits 40 / 60 / 80…) : 24 px, fond `--bg-control`, bord `--border`, survol bord `--accent`, icône 12 px.
- Note en pied de menu : `--fs-xs` muted (« Ou déposez un fichier dans la fenêtre ») ; avertissement en tête : même taille en `--warning` (« Corrigez les erreurs avant d'exporter »).

## Menu contextuel (clic droit sur le canevas)

Même panneau que `Popover` (z-index 600, largeur mini 210 px), lignes de 28 px, icône 14 px dans une colonne de 14 px, `--fs-sm`, désactivé en `--text-disabled`, séparateurs `--divider`. Se ferme sur Échap, clic extérieur, molette ou perte du focus de la fenêtre.

## `Dialog` — fenêtre modale

`<dialog>` natif ouvert par `showModal()` : largeur `min(560–620px, 100vw − 32px)`, fond `--bg-elevated`, rayon 6 px, `--shadow-pop`, voile `--backdrop`. En-tête : titre 14 px + bouton fermer `X` 16 px (quiet icon), padding 12 × 16 px, bord bas. Corps : padding `12px 16px 16px`, `--fs-md`, interligne 1,55, hauteur ≤ 70–78 vh avec défilement. Pied facultatif : actions à droite (Annuler en `btn`, validation en `btn primary`), bord haut. Sous-titres `h3` en petites capitales.

Assistant en plusieurs étapes (premier lancement) : points d'avancement 8 px dans l'en-tête (`--border-strong`, fait `--text-muted`, courant `--accent`), corps de 300 px de haut au moins, phrase d'introduction en `--fs-lg`, boutons Précédent / Suivant en pied.

## `Alerts` / liste de messages

- Erreur bloquante : ligne `CircleX` 14 px rouge sur fond `--danger-tint`, texte `--danger-text`.
- Avertissement : `TriangleAlert` 14 px orange, texte primary, **sans fond**.
- Information : `Info` 13–14 px cyan, texte secondary ; **repliées** derrière « ▸ ⓘ 3 remarques — aperçu de la première… ».
- Lignes : padding 5 × 8 px, `--fs-sm`, interligne 1,45, icône alignée sur la première ligne (`margin-top: 1px`). `aria-live="polite"`.
- Variante compacte (inspecteur FabVents) : grille `14px 1fr`, gap 6 px, sans fond pour les erreurs mais texte `--danger-text`.

## `Tag` — étiquette

Icône 11 px + texte `--fs-xs` 600, padding 1 × 5 px, rayon 3 px, couleur de sens sur sa teinte. Passe en icône seule quand la place manque (requête de conteneur `@container (max-width: 290px)`), le texte restant dans l'infobulle. Exemples FabSpeed : Testé (`FlaskConical`, cyan), Validé (`ShieldCheck`, vert), Calculé (`Calculator`, gris), Modifié (`Pencil`, violet), Dérivé (`GitBranch`, orange).

## Barre d'outils de l'espace de travail

- Bouton d'outil : 30 × 28 px, transparent, icône 16 px secondary ; survol `--bg-hover` ; **actif : fond `--bg-selected`, icône `--accent-text`** ; désactivé opacité 0,35. Groupe radio, 2 px d'écart.
- Infobulle d'outil sur deux lignes : « Nom (raccourci) » puis une phrase d'usage.
- **Onglets** : texte `--fs-md` 500 muted, 12 px d'écart, pleine hauteur de la barre ; actif : texte primary + **soulignement 2 px `--accent`** ; compteur à droite du nom : pastille `--neutral-tint`, rayon 8 px, `--fs-xs`, chiffres tabulaires, masquée à zéro.
- Zoom : pourcentage tabulaire centré dans 48 px (100 % = taille réelle à l'écran, 96 px par pouce CSS).

## Curseur (`input type="range"`)

Curseur natif (FabSpeed, réglage du pas latéral ; FabWeave, avancement de l'impression) : `flex: 1; min-width: 0; accent-color: var(--accent)`, accompagné de sa valeur (champ ou lecture tabulaire). Clavier natif : flèches, Origine, Fin. Vertical (coupe en Z de FabWeave) : `writing-mode: vertical-lr; direction: rtl` (le bas = la valeur basse), dans un panneau de 44 px de large, valeur en haut, unité en bas, `--fs-xs`.

**Barre de lecture** d'une vue (FabWeave, avancement) : panneau flottant en bas de la vue, pleine largeur à `--s2` des bords, fond `--bg-panel`, bord `--border`, rayon 4 px ; bouton lecture / pause (`Play` / `Pause` 15 px, `btn quiet icon`), curseur, lecture tabulaire `--fs-xs` secondary à droite (« 62 % · 3 h 12 min / 7 h 27 min · Z 48,2 mm »). La légende de la vue remonte au-dessus de la barre.

## Liste d'éléments (inspecteur)

Lignes de 26 px, padding 0 4 px, rayon 4 px : icône du type, nom (secondary → primary au survol), valeur courte à droite en muted (`Ø 4,3 mm`). Sélection : `--bg-selected`. En dessous, boutons d'ajout `btn` avec icône (« + Trou de fixation », « ▢ Zone rectangulaire »).

## Mesures (`dl`)

Grille `1fr auto`, gap 6 × 8 px : terme `--fs-sm` secondary, valeur à droite tabulaire ; la mesure clé en `--fs-lg` 600 ; une mesure hors tolérance en `--warning` (avec infobulle qui dit pourquoi).

## Cellule de résultat (FabSpeed)

Bloc sans cadre, séparé de ses voisins par un trait vertical `--divider`, padding 8 × 12 px : en tête libellé en petites capitales + ⓘ + actions mini (20 px, copie visible au survol seulement) + `Tag` d'origine ; puis le chiffre en 28 px 600 avec son unité en `--fs-md` secondary ; puis une ligne secondaire `--fs-sm` muted. Cliquer le chiffre le rend modifiable ; une valeur saisie prend la couleur « utilisateur » et un bouton ↺ pour revenir à la proposition. Aperçu d'une variante (survol d'un levier) en `--success`.

## Encart du document (barre d'application)

`label.pbox` de 28 px : fond `--bg-panel`, bord `--border` (survol `--border-strong`, focus `--accent`), icône `FileText` 14 px, libellé « Projet » `--fs-sm` secondary, nom en **600 primary** modifiable sur place. **La largeur suit le texte** (texte invisible `.sizer` et `input` dans la même case de grille), 460 px au plus (280 px sous 1100 px). FabSpeed y montre la machine active.

## Toasts

En bas au centre (36 px au-dessus du bas), fond `--tooltip-bg`, texte `--tooltip-text` `--fs-sm`, padding 8 × 14 px, rayon 6 px, `--shadow-pop`, apparition `rise` 160 ms (6 px vers le haut), 2,6 s, `pointer-events: none`, `aria-live="polite"`.

## Infobulle (`use:tip`)

Bulle unique partagée, fixe, max 280 px, padding 6 × 8 px, rayon 4 px, `--tooltip-bg` / `--tooltip-text`, 500 11,5 px, `white-space: pre-line` (retour à la ligne par `\n`). Délai **380 ms** au survol ou au focus clavier, fondu 120 ms, sous l'élément (au-dessus s'il manque de place), masquée au clic. Fournit aussi l'`aria-label` d'un bouton sans texte.

## `kbd`

`display: inline-block; padding: 1px 6px; bord --border-strong; rayon 4 px; fond --bg-control; 500 --fs-sm`. Tableau des raccourcis de l'aide : grille `120–150px 1fr`.

## Voile de dépôt

Pendant un glisser de fichier au-dessus de la fenêtre : couche fixe plein écran (z 800), fond `--backdrop`, **bord pointillé 2 px `--accent`**, une phrase `--fs-lg` au centre qui dit ce qu'on peut déposer (« Déposer un projet FabVents (.json, .svg) ou une image (contrôleur Image) »).

## Barre d'état

Pastille 7 px (vert prêt, orange chargement, rouge hors service) + texte ; séparateurs 1 × 12 px `--border-strong` ; à gauche état, version, compteurs, durée du calcul ; à droite contexte (curseur, machine) ; un état transitoire important en `--warning` (« calcul au relâchement »).

## Pictogrammes maison

Quand Lucide n'a pas l'icône (formes de perforation, dispositions, opérations d'usinage, fraises) : SVG 24 × 24, `fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"`, couleur héritée du texte, taille passée en propriété. Ils doivent se confondre avec Lucide.

## Motifs complémentaires

Pour les lignes de tâche, tableaux filtrables, encarts de source ou de proposition et interfaces d’assistance, utiliser les spécifications de [beautiful-ui.md](beautiful-ui.md). Le kit fournit TaskList, RecordsTable, ContextCard, ProposalCard et AssistantPanel ; les autres motifs sont à implémenter selon le besoin.
