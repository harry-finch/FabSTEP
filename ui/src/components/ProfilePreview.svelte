<script lang="ts">
  // Rendu commun des courbes préparées hors fil UI ; aucune géométrie n’est ajustée ici.
  import Disclosure from './Disclosure.svelte';
  import { i18n,t,type Key } from '../lib/i18n.svelte';
  import type { FaceProfileGeometry } from '../lib/face-profile-geometry';
  import type { ProfileFrame } from '../lib/types';
  let {geometry,frame,holes,id='dxf1-frame',previewKey='faceProfile.preview',frameKey='faceProfile.frame',mappingKey='faceProfile.mapping'}:{geometry:FaceProfileGeometry|null;frame:ProfileFrame;holes:number;id?:string;previewKey?:Key;frameKey?:Key;mappingKey?:Key}=$props();
  const number=(value:number)=>new Intl.NumberFormat(i18n.locale === 'fr'?'fr-FR':'en-GB',{maximumSignificantDigits:7}).format(value);
  const vector=(values:number[])=>values.map(number).join(' ; ');
</script>
{#if geometry}<svg viewBox={geometry.viewBox} role="img" aria-label={t(previewKey)}>
  <title>{t(previewKey)}</title>
  <g transform="scale(1 -1)">{#each geometry.paths as path}<path d={path.d} class:hole={!path.outer} data-solid-id={path.solidId} data-component-id={path.componentId}/>{/each}</g>
</svg>
<div class="metric"><span>{t('faceProfile.axes')}</span><strong>X → · Y ↑</strong></div>
<div class="metric"><span>{t('workspace.dimensions')}</span><strong>{number(geometry.width)} × {number(geometry.height)} mm</strong></div>
<div class="metric"><span>{t('faceProfile.holes')}</span><strong>{holes}</strong></div>
<p class="muted">{t('faceProfile.scale')}</p>
{/if}
<Disclosure {id} label={t(frameKey)}>
  <p class="muted">{t(mappingKey)}</p>
  <div class="metric"><span>{t('faceProfile.origin')}</span><strong>{vector(frame.origin_mm)} mm</strong></div>
  <div class="metric"><span>{t('faceProfile.xAxis')}</span><strong>{vector(frame.x_axis)}</strong></div>
  <div class="metric"><span>{t('faceProfile.yAxis')}</span><strong>{vector(frame.y_axis)}</strong></div>
  <div class="metric"><span>{t('faceProfile.normal')}</span><strong>{vector(frame.normal)}</strong></div>
  {#if geometry}<div class="metric"><span>{t('faceProfile.xRange')}</span><strong>{number(geometry.bounds[0][0])} → {number(geometry.bounds[1][0])} mm</strong></div>
  <div class="metric"><span>{t('faceProfile.yRange')}</span><strong>{number(geometry.bounds[0][1])} → {number(geometry.bounds[1][1])} mm</strong></div>{/if}
</Disclosure>
<style>
  svg{display:block;width:100%;height:var(--face-profile-preview-height);background:var(--view-bg);border:1px solid var(--border);border-radius:var(--radius)}
  path{fill:none;stroke:var(--accent);stroke-width:1.5;vector-effect:non-scaling-stroke}path.hole{stroke:var(--text-primary)}
  .metric{display:flex;justify-content:space-between;align-items:center;gap:var(--s2);min-height:var(--control-h);font-size:var(--fs-xs);color:var(--text-secondary)}
  .metric strong{text-align:right;max-width:70%;overflow-wrap:anywhere;font-weight:500;color:var(--text-primary);font-variant-numeric:tabular-nums}
  .muted{margin:0;font-size:var(--fs-xs);line-height:1.5;color:var(--text-muted)}
</style>
