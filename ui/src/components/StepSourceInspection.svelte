<script lang="ts">
  // Source démontrée de la sélection STEP ; les faces natives restent sélectionnées au retour.
  import { ArrowLeft, X } from '@lucide/svelte';
  import Section from './Section.svelte';
  import Disclosure from './Disclosure.svelte';
  import { t } from '../lib/i18n.svelte';
  import type { StepSelection } from '../lib/step-selection';
  let { selection, onback, onclear }:{ selection:StepSelection; onback:()=>void; onclear:()=>void } = $props();
</script>
<Section id="step-source-inspection" title={t('stepInspection.sourceZone')}>
  <div class="actions"><button class="btn" onclick={onback}><ArrowLeft size={14}/>{t('stepInspection.backToStep')}</button><button class="btn quiet icon" title={t('inspection.clear')} aria-label={t('inspection.clear')} onclick={onclear}><X size={14}/></button></div>
  <Disclosure id="ux4-source-correspondence" label={t('workspace.resultDetails')}>
    <div class="metric"><span>{t('stepInspection.selectedFaces')}</span><strong>{selection.faces.length}</strong></div>
    {#if selection.source.status === 'exact'}
      <div class="metric"><span>{t('workspace.triangles')}</span><strong>{selection.source.triangleCount}</strong></div>
      <div class="metric"><span>{t('workspace.bodies')}</span><strong>{selection.source.bodies.length}</strong></div>
    {/if}
    <p>{t('stepInspection.exactCorrespondence')}</p>
    <p>{t('stepInspection.scope')}</p>
  </Disclosure>
  <p>{t('stepInspection.sourcePickHint')}</p>
</Section>
<style>
  .metric{display:flex;align-items:center;justify-content:space-between;gap:var(--s2);min-height:var(--control-h);font-size:var(--fs-sm);color:var(--text-secondary)}
  .metric strong{color:var(--text-primary);font-weight:500;font-variant-numeric:tabular-nums}.actions{display:flex;gap:var(--s1)}.actions .btn:first-child{flex:1}
  p{margin:0;color:var(--text-muted);font-size:var(--fs-xs);line-height:1.5}
</style>
