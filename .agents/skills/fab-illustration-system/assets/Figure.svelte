<script lang="ts">
  // Illustration au trait (figure Hairline) : dessin isométrique d'un seul
  // trait, au repos tant qu'on ne le touche pas, qui répond au pointeur.
  // `figure` est une fonction du catalogue (`sieve`, `drawer`… importées de
  // `@lucasmarkes/hairline`) ou une figure maison (`made('nom')`, voir
  // `lib/figures.ts`). Les couleurs viennent des jetons `--hairline-*` de
  // `app.css` ; `surface` dit sur quel fond la figure est posée, parce que
  // ses plaques sont remplies de ce fond. `label` est sa description pour les
  // lecteurs d'écran : toujours une clé traduite (`t('figure.…')`).
  // `params` ne sert qu'à une figure maison paramétrique (aperçu d'un objet
  // en cours de saisie) : elle se redessine quand ils changent.
  import type { Figure } from '@lucasmarkes/hairline';
  import type { FigureOptions } from '../lib/figures';

  let {
    figure,
    label,
    surface = 'workspace',
    width = 240,
    intensity = 0.5,
    play = false,
    params,
  }: {
    figure: (el: HTMLElement, options?: FigureOptions) => Figure;
    label: string;
    surface?: 'workspace' | 'panel' | 'elevated' | 'chrome';
    /** Largeur en px ; la hauteur suit (5:4). 240 par défaut, 160 au moins, 320 au plus dans l'application. */
    width?: number;
    /** Force de la réponse au pointeur, de 0 à 1. */
    intensity?: number;
    /** Parcours automatique en boucle : premier lancement et site seulement. */
    play?: boolean;
    /** Paramètres d'une figure maison paramétrique (objet simple, sérialisable). */
    params?: unknown;
  } = $props();

  // `update` et `destroy` d'une figure sont ceux d'une action Svelte.
  const draw = (el: HTMLElement, options: FigureOptions) => figure(el, options);
</script>

<div
  class="figure"
  style:width="{width}px"
  style:--hairline-plate="var(--bg-{surface})"
  use:draw={{ label, intensity, play, params }}
></div>

<style>
  .figure {
    max-width: 100%;
    flex: none;
  }
</style>
