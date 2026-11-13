# Canevas, schémas et dessins techniques

Le centre de l'écran montre le travail : un canevas éditable (FabVents), un schéma de coupe (FabSpeed), un aperçu de parcours (FabCAM), une vue 3D du parcours (FabWeave). Mêmes règles visuelles que le reste de l'interface.

## Couleurs : uniquement des jetons

Le code de dessin (Canvas 2D, SVG, WebGL) ne contient **aucune couleur en dur**. Pour un Canvas 2D :

```ts
// Jetons lus au montage et à chaque changement de thème.
const TOKENS = ['canvas-bg', 'plate-fill', 'plate-edge', 'ctrl', 'ctrl-edge', 'ctrl-active', 'tooltip-bg', 'tooltip-text', 'font'];
let colors: Record<string, string> = {};
function readColors() {
  const cs = getComputedStyle(document.documentElement);
  colors = Object.fromEntries(TOKENS.map((k) => [k, cs.getPropertyValue(`--${k}`).trim()]));
}
$effect(() => {
  void theme.applied; // relit les jetons quand le thème change
  readColors();
  // … construire la scène et demander une image
});
```

En SVG inline, utiliser directement `fill="var(--scene-danger)"` ou des classes stylées par jetons.

## Rendu

- **Une image par trame au plus** : l'effet construit un instantané de la scène et appelle `requestAnimationFrame` s'il n'y en a pas déjà un en attente.
- **Écrans haute densité** : taille du canevas = taille CSS × `devicePixelRatio`, puis `ctx.setTransform(dpr, 0, 0, dpr, 0, 0)` ; on dessine en pixels CSS.
- Un chemin par famille d'objets (tous les cercles en un `beginPath`, puis un `fill`) : des dizaines de milliers d'objets restent fluides.
- **Repère** : le moteur travaille en mm, origine en bas à gauche, Y vers le haut (convention CAO). Le canevas retourne l'axe Y à l'affichage ; toutes les conversions passent par une paire `toScreen` / `toWorld`.
- **Zoom** : 100 % = taille réelle à l'écran (96 px CSS par pouce, soit 3,78 px par mm). Zoom à la molette autour du curseur ; cadrage automatique tant que l'utilisateur n'a pas touché à la vue.

## Hiérarchie visuelle sur le canevas

| Élément | Dessin |
| --- | --- |
| Fond | `--canvas-bg` (= fond de l'espace de travail) |
| Pièce (plaque, brut) | Aplat neutre gris moyen (`--plate-fill` `#4b4f55` en sombre, `#b9bdc3` en clair), contour 1 px `--plate-edge` |
| Vides (trous, matière enlevée) | Couleur du fond : le vide « laisse voir » le fond. Règle *evenodd* pour les îlots |
| Zones de construction (marge, zone utile) | Trait pointillé fin `[4, 4]` ou `[2, 3]`, 1 px, blanc / noir translucide (`--plate-margin`) |
| Objets de l'utilisateur (contrôleurs, tracés) | **Blanc cassé** `--ctrl` 1,5 px sur un **liseré sombre** `--ctrl-edge` de 4 px : lisible sur n'importe quel fond |
| Objet survolé | Trait 2 px |
| Objet sélectionné | Trait et poignées `--ctrl-active` (= accent bleu), remplissage `--ctrl-active-fill` (bleu 6 %) |
| Portée, influence | Trait pointillé `[5, 4]` `--ctrl-reach`, remplissage `--ctrl-reach-fill` (5 %) ; trait plein si bord franc |
| Objets des autres onglets | `--ctrl-dim` (30 %) : présents mais en retrait |
| Aperçu d'un tracé en cours, règle | `--canvas-ghost` (bleu 70 %) |
| Zones réservées | Pointillé, et bande de garde translucide |

## Poignées

- Disque de rayon 6 px (`HANDLE`) pour le point principal et le centre, 4,5 px pour les autres, 7 px au survol ; carré 8 × 8 px pour une poignée de taille ou de portée.
- Remplissage `--ctrl`, contour 1,5 px `--ctrl-edge` ; nœud actif ou objet sélectionné : `--ctrl-active`.
- Zone de prise plus large que le dessin (rayon 9 px, `HIT`) : on attrape sans viser.

## Étiquettes sur le canevas

Pastille `--tooltip-bg`, texte `--tooltip-text` 600 11 px dans la police de l'interface, rayon 3 px, hauteur 18 px, padding 6 px, décalée de 10 px en haut à droite du point : valeur cible d'un contrôleur (« 4,5 mm », « 30° »), cote de la règle (« 42,3 mm · 15° »).

## Vue 3D (WebGL)

FabWeave montre le parcours de la buse en 3D avec **three.js** (WebGL 2, ADR-0003 de FabWeave, composant `Viewport.svelte`) : plateau à la taille de la machine, grille de 10 mm, parcours en lignes dégradées selon la hauteur, déplacements en retrait, Z vers le haut. **Rendu à la demande** seulement : pas de boucle d'animation, un changement de caméra, de scène ou de taille demande une image par `requestAnimationFrame`, une seule en attente. Les couleurs sont lues dans les jetons `--view-*` (fond, plateau, bord, grille) et `--path-*` (bas et haut du dégradé, déplacements), définis dans les deux thèmes, convertis en `THREE.Color` et **relus au changement de thème** comme pour un Canvas 2D ; plateau et parcours restent neutres. La vue se **cadre seule** sur chaque calcul tant que l'utilisateur ne l'a pas bougée (orbite, déplacement, zoom vers le curseur) ; changer de modèle rétablit le cadrage automatique, et **`F`** (ou le bouton `Focus` en haut à droite de la vue) recadre.

**Avancement et coupe** (VIEW-08) : la barre de lecture montre le parcours jusqu'au point courant par **plages de dessin** (`setDrawRange`, `instanceCount`), sans rien reconstruire ; une **buse simulée** neutre et translucide (`--view-nozzle`, arêtes `--view-nozzle-edge`) marque le point courant tant qu'on n'est pas au bout ; la **coupe en Z** est un plan de coupe three.js (`localClippingEnabled`, shader des boudins compris), réglé par un curseur vertical à droite.

## Schémas (SVG)

Un schéma montre le travail **à l'échelle**, en projection droite (coupe, vue de dessus), avec ses cotes et ses textes. Il se dessine **au trait**, dans le langage des illustrations de la famille (skill `fab-illustration-system`), sans en être une : il porte des valeurs, des mots et des couleurs de sens. Référence : le schéma de coupe de FabSpeed (`CutScene.svelte`).

- **Le trait** : 1 px constant (`vector-effect: non-scaling-stroke`), extrémités et jointures rondes, en quatre poids. La classe `.linework`, posée sur la racine du schéma, donne les mêmes gris que les figures : `--hairline-hi` (vif), `--hairline-edge` (silhouettes, cotes, flèches), `--hairline-mid` (traits courants), `--hairline-lo` (en retrait, lignes de passes en pointillé). **Un seul élément vif** : le sujet du schéma (la fraise).
- **Les formes** : au contour, remplies de la couleur du fond pour cacher ce qui est derrière, coins arrondis de 2 px. Ni grille de fond, ni aplat gris, ni hachures.
- **La couleur, trois usages, pas un de plus** :
  1. *la matière* : la pièce non usinée prend la couleur de sa matière (une donnée, comme une pastille) en teinte légère (`--scene-material-tint` : 0,18 en sombre, 0,28 en clair) ; ce qui est enlevé reste vide ;
  2. *l'état de ce qui se passe* : la zone en train d'être coupée prend la couleur de sens du verdict affiché ailleurs en toutes lettres (vert sain, orange à surveiller), en petit : une tranche, un secteur ;
  3. *le danger* : rouge (`--scene-danger`), pour ce qui bloque.
  Jamais l'accent, jamais l'identité ; flèches et textes restent neutres.
- **Remplir le cadre** : le dessin prend les proportions de sa zone (mesurée) et l'échelle occupe toute la hauteur ; ce qui n'a pas d'échelle propre (la longueur d'une plaque) s'allonge plutôt que de laisser des marges.
- **Le mouvement** dit quelque chose (le défilement des goujures donne le sens d'évacuation des copeaux) et s'arrête sous `prefers-reduced-motion`.
- **Textes** : cotes et légendes par `t('…')`, en `--text-secondary` / `--text-muted`.

## Export d'image

Un export PNG « de présentation » reprend la pièce sur fond transparent, à une résolution d'atelier (300 ppp), plafonnée (8 192 px de côté), avec une couleur de pièce fixe et neutre (`#3a3d42` dans FabVents) indépendante du thème. C'est la seule couleur en dur tolérée : elle appartient au fichier produit, pas à l'interface.
