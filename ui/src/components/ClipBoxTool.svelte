<script lang="ts">
  import Box from '@lucide/svelte/icons/box';
  import Sliders from '@lucide/svelte/icons/sliders-horizontal';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import { i18n, t } from '../lib/i18n.svelte';
  import { axes, type Axis, type Limit } from '../lib/clip-box';
  import type { Box3 } from 'three';

  let { box, ontoggle, onreset, onlimit }: {
    box: Box3 | null;
    ontoggle: () => void;
    onreset: () => void;
    onlimit: (axis: Axis, side: Limit, value: number) => void;
  } = $props();
  let editing = $state(false);
  const sides = ['min', 'max'] as const;
  function edit(input: HTMLInputElement, axis: Axis, side: Limit) {
    if (!box) return;
    const raw = input.value.trim();
    const parsed = Number(i18n.locale === 'fr' ? raw.replace(',', '.') : raw);
    if (raw && Number.isFinite(parsed) && parsed !== box[side][axis]) onlimit(axis, side, parsed);
    input.value = display(box[side][axis]);
  }
  function display(value: number, exact = false) {
    const magnitude = Math.abs(value);
    const text = exact ? String(value)
      : magnitude > 0 && (magnitude < 0.0001 || magnitude >= 1e8)
        ? value.toExponential(3)
        : String(Number(value.toFixed(4)));
    return i18n.locale === 'fr' ? text.replace('.', ',') : text;
  }
</script>
<div class="clip-tool">
  <div class="switches">
    <button class="btn" class:active={!!box} aria-pressed={!!box} onclick={() => { editing = false; ontoggle(); }}>
      <Box size={14}/>{t('clipBox.toggle')}
    </button>
    {#if box}
      <button class="btn icon" title={t('clipBox.adjust')} aria-label={t('clipBox.adjust')} aria-expanded={editing} onclick={() => editing = !editing}>
        <Sliders size={14}/>
      </button>
    {/if}
  </div>
  {#if box && editing}
    <fieldset>
      <legend>{t('clipBox.bounds')}</legend>
      <div class="limits">
        <span></span><span>{t('clipBox.min')}</span><span>{t('clipBox.max')}</span>
        {#each axes as axis}
          <span>{axis.toUpperCase()}</span>
          {#each sides as side}
            <input class="input tabular" inputmode="decimal" aria-label={axis.toUpperCase() + ' ' + t(side === 'min' ? 'clipBox.min' : 'clipBox.max')}
              value={display(box[side][axis])} title={display(box[side][axis], true)}
              onfocus={event => {
                event.currentTarget.value = display(box![side][axis], true);
                event.currentTarget.select();
              }}
              onblur={event => event.currentTarget.value = display(box![side][axis])}
              onchange={event => edit(event.currentTarget, axis, side)}
              onkeydown={event => {
                if (event.key === 'Enter') event.currentTarget.blur();
                if (event.key === 'Escape') { event.stopPropagation(); event.currentTarget.value = display(box![side][axis], true); event.currentTarget.blur(); }
              }}/>
          {/each}
        {/each}
      </div>
      <button class="btn quiet" onclick={onreset}><RotateCcw size={14}/>{t('clipBox.reset')}</button>
    </fieldset>
  {/if}
</div>
<style>
  .clip-tool { position:absolute; top:var(--s2); right:var(--s2); display:grid; gap:var(--s1); justify-items:end; }
  .switches { display:flex; gap:var(--s1); }
  .active { border-color:var(--accent); color:var(--accent-text); background:var(--bg-panel); }
  fieldset { background:var(--bg-panel); border:1px solid var(--border-strong); border-radius:var(--radius); margin:0; padding:var(--s2); display:grid; gap:var(--s2); }
  legend { background:var(--bg-panel); font-size:var(--fs-xs); color:var(--text-secondary); }
  .limits { display:grid; grid-template-columns:auto repeat(2, 12ch); align-items:center; gap:var(--s1); font-size:var(--fs-xs); }
  input { width:100%; text-align:right; }
</style>
