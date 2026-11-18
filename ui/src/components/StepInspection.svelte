<script lang="ts">
  // Faces natives : sélection multiple, voisinage par arête, provenance et profil DXF.
  import { ArrowRight, Minus, Plus, X } from '@lucide/svelte';
  import Section from './Section.svelte';
  import Disclosure from './Disclosure.svelte';
  import FaceProfile from './FaceProfile.svelte';
  import type { FaceProfileState } from '../lib/face-profile';
  import { t, type Key } from '../lib/i18n.svelte';
  import type { StepSelection } from '../lib/step-selection';
  import type { ContextReveal } from '../lib/context-reveal';
  import type { ResultFaces, StepFamily } from '../lib/types';
  let { selection, document, revealContext, ongrow, onshrink, onclear, onshowsource, profile, profileEligible, canexportprofile, onexportprofile, errorText }: { selection:StepSelection|null; document:ResultFaces; revealContext:ContextReveal; ongrow:()=>void; onshrink:()=>void; onclear:()=>void; onshowsource:()=>void; profile:FaceProfileState; profileEligible:boolean; canexportprofile:boolean; onexportprofile:()=>void; errorText:(error:unknown)=>string } = $props();
  const labels:Record<StepFamily,Key> = { plane:'inspection.plane', cylinder:'inspection.cylinder', cone:'inspection.cone', sphere:'inspection.sphere', torus:'inspection.torus', revolution:'inspection.revolution', height_patch:'inspection.heightPatch', other:'stepInspection.other' };
</script>
<Section id="step-inspection" title={t('stepInspection.title')} {revealContext} revealKey={selection?.faces.length ? `${document.request_id}:${document.source_id}:${[...selection.faces].sort((a,b)=>a-b).join(',')}` : null}>
  {#if selection?.faces.length}
    {#if selection.source.status === 'exact'}
      <button class="btn source-action" title={t('stepInspection.sourceHint')} onclick={onshowsource}><ArrowRight size={14}/>{t('stepInspection.showSource')}</button>
    {:else if selection.source.status === 'unavailable' && selection.source.reason === 'step.provenance_invalid'}
      <p class="muted">{t('stepInspection.invalidCorrespondence')}</p>
    {/if}
    {#if profileEligible}
      <FaceProfile state={profile} canexport={canexportprofile} onexport={onexportprofile} {errorText}/>
    {/if}
  {:else}
    <p class="muted">{t('stepInspection.pickHint')}</p>
  {/if}
  {#if selection?.faces.length}
    <Disclosure id="step-selection-advanced" label={t('workspace.advanced')}>
      <div class="actions">
      <button class="btn" title={t('stepInspection.growHint')} disabled={!selection.hasMore} onclick={ongrow}><Plus size={14}/>{t('inspection.grow')}</button>
      <button class="btn" title={t('stepInspection.shrinkHint')} disabled={!selection.canShrink} onclick={onshrink}><Minus size={14}/>{t('inspection.shrink')}</button>
      <button class="btn quiet icon" title={t('inspection.clear')} aria-label={t('inspection.clear')} onclick={onclear}><X size={14}/></button>
      </div>
    </Disclosure>
  {/if}
  <Disclosure id="ux4-step-selection-details" label={t('workspace.resultDetails')}>
    {#if selection?.faces.length}
      <div class="label" title={t('stepInspection.faceColors')}>{t('stepInspection.families')}</div>
      {#each Object.entries(selection.families) as [family,count]}
        <div class="metric"><span>{t(labels[family as StepFamily])}</span><strong>{count}</strong></div>
      {/each}
    {/if}
    <p class="muted">{t('stepInspection.faceColors')}</p>
    <p class="muted">{t('stepInspection.scope')}</p>
    <p class="muted">{t('guidance.requirement')}</p>
    <p class="muted">{t('guidance.cylinderScope')}</p>
    <p class="muted">{t('guidance.coneScope')}</p>
    {#if !profileEligible}<p class="muted">{t('faceProfile.requirement')}</p>{/if}
    <div class="metric"><span>{t('workspace.deflection')}</span><strong>{document.deflection_mm} mm</strong></div>
    {#if selection?.faces.length}
      <div class="metric"><span>{t('stepInspection.solids')}</span><strong>{selection.solids.map(id => id+1).join(', ')}</strong></div>
      <p class="muted">{t('stepInspection.identitiesHint')}</p>
      <code aria-label={t('stepInspection.identities')}>{selection.faces.map(id => id+1).join(', ')}</code>
    {/if}
    {#if selection?.source.status === 'exact'}
      <p class="muted">{t('stepInspection.exactCorrespondence')}</p>
    {:else}
      <p class="muted">{t(selection?.source.status === 'unavailable' && selection.source.reason === 'step.provenance_invalid' ? 'stepInspection.invalidCorrespondence' : 'stepInspection.noCorrespondence')}</p>
      <code>{selection?.source.status === 'unavailable' ? selection.source.reason : 'step.provenance_unavailable'}</code>
    {/if}
  </Disclosure>
</Section>
<style>
  .source-action{width:100%}
  .metric{display:flex;justify-content:space-between;align-items:center;gap:var(--s2);min-height:var(--control-h);font-size:var(--fs-sm);color:var(--text-secondary)}
  .metric strong{max-width:60%;overflow-wrap:anywhere;font-weight:500;color:var(--text-primary);font-variant-numeric:tabular-nums}
  .actions{display:flex;gap:var(--s1)}.actions .btn{flex:1;padding-inline:var(--s1);min-width:0}.actions .icon{flex:none}
  .muted{margin:0;color:var(--text-muted);font-size:var(--fs-xs);line-height:1.5}.label{font-size:var(--fs-xs);color:var(--text-section);margin-top:var(--s1)}code{display:block;overflow-wrap:anywhere;font-size:var(--fs-xs);color:var(--text-secondary)}
</style>
