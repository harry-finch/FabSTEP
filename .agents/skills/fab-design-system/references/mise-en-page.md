# Mise en page

## Anatomie de la fenêtre

```text
┌▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔ liseré d'identité 2 px ▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔┐
│ [logo] Nom  sous-titre │ Fichier ▾ [▤ Projet  Nom du doc] │ mm|pouces FR|EN ◐☀☾ ? [Exporter ▾] │ 42 px, --bg-chrome
├──────────────────────┬──────────────────────────────────────┬───────────────────────┤
│ PANNEAU DE RÉGLAGE   │ Barre d'outils du canevas (38 px)    │ INSPECTEUR            │
│ (gauche, 296 px)     ├──────────────────────────────────────┤ (droite, 296–320 px)  │
│ ▾ ① PLAQUE           │                                      │ ▾ ÉLÉMENT / CANAL     │
│   Largeur [ 120 ] mm │                                      │   propriétés          │
│   Marge   [   4 ] mm │          ESPACE DE TRAVAIL           │ ▾ MESURES             │
│ ▾ ② DISPOSITION      │        (--bg-workspace, centre)      │   Taux   35,3 %       │
│   …                  │                                      │ ▾ MESSAGES            │
│ ▸ ③ FORME            │                                      │   ⚠ …                 │
├──────────────────────┴──────────────────────────────────────┴───────────────────────┤
│ ● Moteur prêt │ Moteur 0.3.0 │ 673 perforations │ calcul 9 ms        X 12,5 · Y 40 mm │ 24 px, --bg-chrome
└─────────────────────────────────────────────────────────────────────────────────────┘
```

Grille CSS de la fenêtre : `grid-template-rows: auto 1fr auto; height: 100vh`. Zone centrale : `grid-template-columns: <gauche> minmax(0, 1fr) <droite>; min-height: 0` (le `min-height: 0` et le `minmax(0, …)` sont indispensables pour que les panneaux défilent au lieu d'agrandir la page).

| Région | Dimensions | Fond | Bord |
| --- | --- | --- | --- |
| Barre d'application | 42 px de haut, padding 0 12 px, gap 16 px entre groupes | `--bg-chrome` | bas `--border` |
| Liseré d'identité | 2 px, `::before` de la barre, opacité 0,85 | `--identity` | — |
| Panneau gauche | 296 px (FabVents) – 300 px (FabSpeed) ; 264–280 px sous 1100–1279 px | `--bg-panel` | droit `--border` |
| Panneau droit | 296 px (FabVents) – 320 px (FabSpeed) ; 264 px sous 1100 px | `--bg-panel` | gauche `--border` |
| Barre d'outils de l'espace de travail | 38 px, padding 0 8 px, gap 8 px | `--bg-panel` | bas `--border` |
| En-tête de panneau (facultatif) | 32 px, padding 0 12 px, `--fs-sm` 600 secondary | — | bas `--divider` |
| Espace de travail | le reste | `--bg-workspace` / jeton du domaine | — |
| Barre d'état | 24 px, padding 0 12 px, gap 8 px | `--bg-chrome` | haut `--border` |

## Écrans plus petits

- **≤ 1279 px** (FabSpeed, trois colonnes riches) : le panneau droit passe sous l'espace de travail (`grid-template-areas: 'setup ws' 'setup assist'`, hauteur ≤ 300 px, bord haut au lieu du bord gauche).
- **≤ 1100 px** (FabVents, canevas) : panneaux réduits à 264 px, sous-titre de la barre masqué, encart du document limité à 280 px.
- **≤ 860 px** : une seule colonne qui défile (`height: auto; min-height: 100vh`). Lisible, pas optimisé : l'application vise l'ordinateur d'atelier.

## Panneaux

- Un panneau = une pile de **sections repliables** (`Section`), séparées par un trait `--divider`. Éviter les grandes cartes décoratives ; un encart compact peut identifier une source ou une décision.
- Défilement vertical du panneau entier (`overflow-y: auto`), jamais de défilement horizontal : un contenu qui déborde est un défaut.
- Panneau gauche : **ce que l'on fabrique**, dans l'ordre de travail, sections numérotées (① Plaque ② Disposition ③ Forme ④ Bords ⑤ Fabrication ; FabSpeed : machine, matière, fraise, opération).
- Panneau droit : **ce qui est sélectionné, puis ce que l'on obtient** : l'élément ou le canal en tête, puis Mesures, puis Messages. (FabSpeed : l'assistant et les messages.)

## Section

- En-tête 34 px : chevron 14 px (tourne de 90° à l'ouverture, 120 ms), numéro d'étape facultatif, titre en petites capitales ; actions icônes à droite (`actions`), 2 px d'écart.
- Corps : padding `2px 12px 12px`, colonne flex, **gap 8 px** entre lignes.
- Ouverte par défaut ; l'état est mémorisé par poste (`<app>.section.<id>`).

## Lignes de propriété

```text
|← 100 px →|← 8 →|←—— 1fr ——→|← 8 →|← 30 →|
 Largeur ⓘ        [     120 ]       mm
```

- `Field` (non numérique) : `grid-template-columns: 100px minmax(0, 1fr)`, gap 8 px, hauteur mini 28 px.
- `NumField` : `100px minmax(0, 1fr) 30px` (colonne d'unité).
- Un libellé, un contrôle, une unité : **une propriété par ligne**.
- Deux champs courts liés (X / Y, largeur / hauteur) peuvent partager une ligne en deux colonnes égales, toujours alignées sur la même grille.
- **Sélecteur à trois options ou plus** : ne pas le mettre dans un `Field` (la colonne de 100 px le comprime). Écrire un sous-libellé (`--fs-sm`, secondary) puis le `Segmented` sur toute la largeur du panneau.
- Un groupe de réglages secondaires se range sous `Disclosure` (« ▸ Avancé », « ▸ Centre de la zone »).

## Espace de travail

- Toujours centré sur le travail : cadrage automatique tant que l'utilisateur n'a ni zoomé ni déplacé la vue ; `F` recadre.
- Barre d'outils en haut : outils (groupe radio à gauche), séparateur vertical (1 × 18 px, `--border-strong`), onglets, espace extensible, annuler / rétablir, séparateur, zoom (− 100 % + ⤢).
- État vide ou chargement : texte muted centré (`display: grid; place-content: center`), avec une icône 24–32 px au plus et une phrase qui dit quoi faire. Quand l'état vide occupe tout l'espace de travail, une figure au trait de 240 px peut remplacer l'icône (skill `fab-illustration-system`) ; jamais pour un chargement. Dans un panneau, la seule figure admise est l'aperçu paramétrique d'une fiche.
- Aide contextuelle de l'outil ou de l'onglet actif : une phrase en bas à gauche de l'espace de travail (`--fs-sm` muted, 60 % de la largeur au plus, `pointer-events: none`).

## Calques (z-index)

| Couche | z-index |
| --- | --- |
| Menus déroulants (`Popover`) | 500 |
| Menu contextuel | 600 |
| Fenêtres (`<dialog>` natif, couche supérieure du navigateur) | — |
| Voile de dépôt de fichier | 800 |
| Toasts | 900 |
| Infobulle | 1000 |

## Encarts contextuels

Une source, une proposition ou une étape peut être regroupée dans un encart compact quand cela facilite la lecture ou la décision. Employer les surfaces et séparateurs existants, sans ombre de panneau ni grille de grandes cartes ; préserver la priorité du travail central. Voir [beautiful-ui.md](beautiful-ui.md).
