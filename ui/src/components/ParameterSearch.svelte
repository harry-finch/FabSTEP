<script lang="ts">
  // Recherche explicite : proposition liée à la source, application par reconversion complète.
  import { i18n, t } from '../lib/i18n.svelte';
  import { proposedTolerance, type ParameterSearchReport } from '../lib/parameter-search';
  let { enabled, searching, report, currentTolerance, error, errorText, onsearch, onapply }: { enabled:boolean; searching:boolean; report:ParameterSearchReport|null; currentTolerance:number; error:string; errorText:(error:unknown)=>string; onsearch:()=>void; onapply:()=>void } = $props();
  const proposed = $derived(report ? proposedTolerance(report) : null);
  const best = $derived(report && report.best_index !== null ? report.trials[report.best_index] : null);
  const number = (value:number) => new Intl.NumberFormat(i18n.locale, {maximumSignificantDigits:8}).format(value);
</script>
<div class="search">
  <button class="btn" title={t('parameterSearch.hint')} disabled={!enabled} onclick={onsearch}>{t(searching ? 'processing.searchingTitle' : 'parameterSearch.button')}</button>
  {#if error}<p role="alert">{error}</p>{/if}
  {#if report}
    <div role="status">
      {#if proposed !== null && best}
        <strong>{t('parameterSearch.proposal',{value:number(proposed)})}</strong>
        <p>{t('parameterSearch.summary',{faces:best.faces ?? 0,curved:best.curved_faces ?? 0})}</p>
      {:else}<p>{t('parameterSearch.none')}</p>{/if}
    </div>
    {#if proposed !== null}
      {#if proposed === currentTolerance}<p>{t('parameterSearch.same')}</p>
      {:else}<button class="btn" disabled={!enabled} onclick={onapply}>{t('parameterSearch.apply')}</button>{/if}
    {/if}
    <details>
      <summary>{t('parameterSearch.trials')} ({report.trials.length})</summary>
      <p>{t('parameterSearch.hint')}</p>
      <p>{t('parameterSearch.ranking')}</p>
      <ul>{#each report.trials as trial}
        <li><strong>{number(trial.cylinder_fit_tolerance_mm)} mm</strong>
          {#if trial.outcome === 'validated'}
            <span>{t('parameterSearch.summary',{faces:trial.faces ?? 0,curved:trial.curved_faces ?? 0})}</span>
            {#if trial.max_deviation_bound_mm !== null}<span>{t('parameterSearch.bound',{value:number(trial.max_deviation_bound_mm)})}</span>{/if}
            {#if trial.max_sampled_distance_mm !== null}<span>{t('parameterSearch.sampled',{value:number(trial.max_sampled_distance_mm)})}</span>{/if}
          {:else}<span>{t('parameterSearch.rejected')} · {trial.error ? errorText(trial.error) : ''}</span>{/if}
        </li>
      {/each}</ul>
    </details>
  {/if}
</div>
<style>
  .search { display:flex; flex-direction:column; gap:var(--s2); }
  p { margin:0; color:var(--text-muted); font-size:var(--fs-sm); line-height:1.4; }
  strong { color:var(--text-primary); font-size:var(--fs-sm); font-weight:600; }
  summary { min-height:var(--control-h); cursor:pointer; color:var(--text-secondary); font-size:var(--fs-sm); }
  ul { padding:0; margin:var(--s2) 0 0; list-style:none; display:flex; flex-direction:column; gap:var(--s2); }
  li { display:flex; flex-direction:column; gap:var(--s1); border-top:1px solid var(--border); padding-top:var(--s2); overflow-wrap:anywhere; }
  li span { color:var(--text-muted); font-size:var(--fs-sm); }
</style>
