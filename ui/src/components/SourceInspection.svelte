<script lang="ts">
  // Inspecteur source : anneaux topologiques réversibles, sans effet sur la reconstruction.
  import { Minus, Plus, X } from '@lucide/svelte';
  import Section from './Section.svelte';
  import Disclosure from './Disclosure.svelte';
  import { t, type Key } from '../lib/i18n.svelte';
  import type { SourceZone, SurfaceFamily, ZoneInspection } from '../lib/selection';
  let { zone, inspection, bodyName, topologyState, hasMore, ongrow, onshrink, onclear }: { zone:SourceZone|null; inspection:ZoneInspection|null; bodyName:string; topologyState:'pending'|'ready'|'unavailable'; hasMore:boolean; ongrow:()=>void; onshrink:()=>void; onclear:()=>void } = $props();
  const labels:Record<SurfaceFamily,Key> = { plane:'inspection.plane', cylinder:'inspection.cylinder', partial_cylinder:'inspection.partialCylinder', partial_cone:'inspection.partialCone', cone:'inspection.cone', sphere:'inspection.sphere', spherical_octant:'inspection.sphere', linear_blend:'inspection.cone', cross_drill:'inspection.cylinder', torus:'inspection.torus' };
</script>
<Section id="source-inspection" title={t('inspection.title')}>
  {#snippet actions()}
    {#if zone}<button class="btn quiet icon" title={t('inspection.clear')} aria-label={t('inspection.clear')} onclick={onclear}><X size={14}/></button>{/if}
  {/snippet}
  {#if zone}
    <div class="metric"><span>{t('inspection.selectedTriangles')}</span><strong>{zone.triangles.length}</strong></div>
    <Disclosure id="source-selection-advanced" label={t('workspace.advanced')}>
      <div class="actions">
        <button class="btn" title={topologyState === 'ready' ? t('inspection.growHint') : t('inspection.topologyUnavailable')} disabled={topologyState !== 'ready' || !hasMore} onclick={ongrow}><Plus size={14}/>{t('inspection.grow')}</button>
        <button class="btn" title={t('inspection.shrinkHint')} disabled={topologyState !== 'ready' || !zone.ring} onclick={onshrink}><Minus size={14}/>{t('inspection.shrink')}</button>
      </div>
      {#if topologyState !== 'ready'}<p class="muted" role="status">{topologyState === 'pending' ? t('inspection.topologyPending') : t('inspection.topologyUnavailable')}</p>{/if}
    </Disclosure>
    <Disclosure id="selection-provenance" label={t('workspace.resultDetails')}>
      <div class="metric"><span>{t('inspection.sourceBody')}</span><strong title={bodyName}>{bodyName}</strong></div>
      <div class="label">{t('inspection.recognizedFamilies')}</div>
      {#if inspection}
        {#each Object.entries(inspection.families) as [family,count]}<div class="metric"><span>{t(labels[family as SurfaceFamily])}</span><strong>{count}</strong></div>{/each}
      {/if}
      {#if !inspection || inspection.unavailable}<p class="muted">{t('inspection.classificationUnavailable')}</p>{/if}
      <div class="label">{t('inspection.association')}</div>
      <p class="muted">{t('inspection.scope')}</p>
      <p class="muted">{t('inspection.sourceBody')}</p><code>{zone.previewBodyId}</code>
      <p class="muted">{t('inspection.analysisComponents')}</p>
      {#if inspection?.analysisBodyIds.length}{#each inspection.analysisBodyIds as id}<code>{id}</code>{/each}{:else}<p class="muted">{t('inspection.associationUnavailable')}</p>{/if}
    </Disclosure>
  {:else}<p class="muted">{t('inspection.pickHint')}</p>{/if}
  <p class="muted" title={t('inspection.scope')}>{t('inspection.readOnly')}</p>
</Section>
<style>
  .metric{display:flex;justify-content:space-between;align-items:center;gap:var(--s2);min-height:var(--control-h);font-size:var(--fs-sm);color:var(--text-secondary)}
  .metric strong{max-width:60%;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-weight:500;color:var(--text-primary);font-variant-numeric:tabular-nums}
  .actions{display:flex;gap:var(--s1)}.actions .btn{flex:1;padding-inline:var(--s1);min-width:0}
  .muted{margin:0;color:var(--text-muted);font-size:var(--fs-xs);line-height:1.5}.label{font-size:var(--fs-xs);color:var(--text-section);margin-top:var(--s1)}
  code{display:block;overflow-wrap:anywhere;font-size:var(--fs-xs);color:var(--text-secondary)}
</style>
