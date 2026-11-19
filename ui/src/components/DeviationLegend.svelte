<script lang="ts">
  import type { DeviationReport } from '../lib/deviation-map';
  import { i18n, t } from '../lib/i18n.svelte';
  let { report }:{report:DeviationReport} = $props();
  const format = $derived(new Intl.NumberFormat(i18n.locale, {maximumSignificantDigits:3}));
</script>

<div class="legend" role="status">
  <strong>{t('deviation.samples')}</strong>
  <div class="scale" aria-hidden="true"></div>
  <div class="range"><span>0 mm</span><span>{t('deviation.limit', {n:format.format(report.budget_mm)})}</span></div>
  <div>{t('deviation.measured', {min:format.format(report.min_mm), max:format.format(report.max_mm)})}</div>
</div>

<style>
  .legend{position:absolute;bottom:calc(var(--s5) + var(--s3));left:var(--s3);padding:var(--s2);background:var(--bg-panel);border:1px solid var(--divider);border-radius:var(--radius);color:var(--text-secondary);font-size:var(--fs-xs);pointer-events:none;font-variant-numeric:tabular-nums}
  strong{font-weight:500;color:var(--text-primary)}
  .scale{height:var(--s1);margin-top:var(--s2);background:linear-gradient(to right,var(--view-deviation-low),var(--view-deviation-mid),var(--view-deviation-high))}
  .range{display:flex;justify-content:space-between;gap:var(--s4);margin:var(--s1) 0}
</style>
