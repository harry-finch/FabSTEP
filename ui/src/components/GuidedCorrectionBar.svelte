<script lang="ts">
  // Correction contextuelle près du modèle ; seules les familles natives prises
  // en charge sont proposées. La préparation remplace explicitement le résultat.
  import X from '@lucide/svelte/icons/x';
  import Segmented from './Segmented.svelte';
  import { t } from '../lib/i18n.svelte';
  import type { GuidedFamily } from '../lib/types';

  let { count, canguide, canprepare, pending = false, onguide, onprepare, onclear }: {
    count:number;
    canguide:boolean;
    canprepare:boolean;
    pending?:boolean;
    onguide:(family:GuidedFamily)=>void;
    onprepare:()=>void;
    onclear:()=>void;
  } = $props();
  let family = $state<GuidedFamily>('auto');
  function tryFamily() {
    if (pending || !canguide) return;
    const submittedFamily = family;
    onguide(submittedFamily);
  }
</script>

<div class="correction-bar" role="group" aria-label={t('guidance.correction')}>
  <span class="selection-count">{count === 1 ? t('guidance.oneFace') : t('guidance.faceCount', {n:count})}</span>
  {#if canguide}
    <fieldset class="family-choices" disabled={pending}>
      <Segmented
        options={[{id:'auto' as const,label:t('guidance.autoChoice'),hint:t('guidance.autoScope')},{id:'plane' as const,label:t('inspection.plane'),hint:t('guidance.planeScope')},{id:'cylinder' as const,label:t('inspection.cylinder'),hint:t('guidance.cylinderScope')},{id:'cone' as const,label:t('inspection.cone'),hint:t('guidance.coneScope')},{id:'sphere' as const,label:t('inspection.sphere'),hint:t('guidance.sphereScope')},{id:'torus' as const,label:t('inspection.torus'),hint:t('guidance.torusScope')}]}
        value={family}
        onchange={value => family=value}
        label={t('guidance.family')}
      />
    </fieldset>
    <button class="btn primary" disabled={pending} title={t('guidance.scope')} onclick={tryFamily}>{t('guidance.tryFamily')}</button>
  {:else if canprepare}
    <button class="btn" disabled={pending} onclick={onprepare} title={t('guidance.prepareHint')}>{t('guidance.prepare')}</button>
    <span class="hint">{t('guidance.prepareHint')}</span>
  {:else}
    <span class="hint" title={t('guidance.requirement')}>{t('guidance.unavailable')}</span>
  {/if}
  <button class="btn quiet icon clear" title={t('inspection.clear')} aria-label={t('inspection.clear')} onclick={onclear}><X size={14}/></button>
</div>

<style>
  .correction-bar{display:flex;align-items:center;flex-wrap:wrap;gap:var(--s2);padding:var(--s1) var(--s2);background:var(--bg-panel);border-bottom:1px solid var(--divider);font-size:var(--fs-sm);color:var(--text-secondary)}
  .selection-count{font-variant-numeric:tabular-nums;color:var(--text-primary)}
  .family-choices{display:flex;min-width:0;margin:0;padding:0;border:0}
  .hint{flex:1;min-width:0;font-size:var(--fs-xs);line-height:1.5;color:var(--text-muted)}
  .clear{margin-left:auto}
</style>
