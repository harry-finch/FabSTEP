<script lang="ts">
  // Ligne de propriété numérique : LIBELLÉ | VALEUR | UNITÉ.
  // - Saisie selon la langue (virgule décimale en français), validée à la
  //   sortie du champ ou sur Entrée, Échap annule.
  // - Flèches haut / bas : ± pas (Maj × 10, Alt ÷ 10).
  // - Glisser horizontalement sur le libellé fait défiler la valeur
  //   (Maj × 10, Alt ÷ 10) ; `onbegin` / `onend` encadrent le geste pour
  //   n'en faire qu'une étape d'annulation.
  // Les longueurs sont reçues et rendues en mm ; l'affichage suit `ui.units`.
  // `invalid` souligne le champ en rouge (erreur du moteur sur ce champ).
  // `onreset` (FabWeave) : quand la valeur diffère de la valeur proposée
  // (`modified`), un ↺ discret à côté du libellé y revient ; son infobulle
  // (`resetHint`) donne la valeur proposée.
  import { Info, RotateCcw } from '@lucide/svelte';
  import { tip } from '../lib/tip';
  import { ui } from '../lib/ui.svelte';
  import { i18n, intlLocale } from '../lib/i18n.svelte';
  import { fromDisplayLength, lengthUnit, toDisplayLength } from '../lib/format';

  type Kind = 'length' | 'angle' | 'count' | 'percent' | 'plain';

  let {
    label,
    value,
    onchange,
    kind = 'length',
    step,
    min = -Infinity,
    max = Infinity,
    decimals,
    unit,
    hint,
    invalid = false,
    disabled = false,
    onbegin,
    onend,
    modified = false,
    onreset,
    resetHint,
  }: {
    label: string;
    value: number;
    onchange: (v: number) => void;
    kind?: Kind;
    /** Pas en unités affichées. */
    step?: number;
    /** Bornes en unités du modèle (mm, degrés…) : bornes de saisie, pas une correction cachée. */
    min?: number;
    max?: number;
    decimals?: number;
    unit?: string;
    hint?: string;
    invalid?: boolean;
    disabled?: boolean;
    onbegin?: () => void;
    onend?: () => void;
    modified?: boolean;
    onreset?: () => void;
    resetHint?: string;
  } = $props();

  const uid = `nf-${Math.random().toString(36).slice(2, 9)}`;
  const isLen = $derived(kind === 'length');
  const inch = $derived(isLen && ui.units === 'in');
  const shown = (v: number) => (isLen ? toDisplayLength(v, ui.units) : kind === 'percent' ? v * 100 : v);
  const model = (v: number) => (isLen ? fromDisplayLength(v, ui.units) : kind === 'percent' ? v / 100 : v);
  const dec = $derived(decimals ?? (kind === 'count' ? 0 : inch ? 3 : isLen ? 2 : 1));
  const dstep = $derived(step ?? (kind === 'count' ? 1 : inch ? 0.01 : isLen ? 0.1 : 1));
  const shownUnit = $derived(unit ?? (isLen ? lengthUnit(ui.units) : kind === 'angle' ? '°' : kind === 'percent' ? '%' : ''));

  const fmt = (v: number) =>
    new Intl.NumberFormat(intlLocale(), { maximumFractionDigits: dec, useGrouping: false }).format(v);

  let text = $state('');
  let focused = $state(false);
  $effect(() => {
    if (!focused) text = Number.isFinite(value) ? fmt(shown(value)) : '';
  });

  function parse(raw: string): number | undefined {
    const compact = raw.trim().replace(/\s/g, '');
    const s = i18n.locale === 'fr' ? compact.replace(',', '.') : compact.replace(/,/g, '');
    if (s === '') return undefined;
    const n = Number(s);
    return Number.isFinite(n) ? n : undefined;
  }

  const clampModel = (v: number) => Math.min(max, Math.max(min, v));
  const roundShown = (v: number) => {
    const k = 10 ** dec;
    return Math.round(v * k) / k;
  };

  function commit() {
    const v = parse(text);
    if (v === undefined) {
      text = fmt(shown(value));
      return;
    }
    const m = clampModel(model(v));
    if (m !== value) onchange(m);
    text = fmt(shown(m));
  }

  function key(e: KeyboardEvent) {
    const el = e.currentTarget as HTMLInputElement;
    if (e.key === 'Enter') el.blur();
    if (e.key === 'Escape') {
      text = fmt(shown(value));
      el.blur();
    }
    if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.preventDefault();
      const base = parse(text) ?? shown(value);
      const k = e.shiftKey ? 10 : e.altKey ? 0.1 : 1;
      const next = clampModel(model(roundShown(base + (e.key === 'ArrowUp' ? 1 : -1) * dstep * k)));
      text = fmt(shown(next));
      onchange(next);
    }
  }

  // Défilement par le libellé.
  let scrub: { x: number; start: number; pointer: number } | null = null;
  function scrubStart(e: PointerEvent) {
    if (disabled || e.button !== 0) return;
    scrub = { x: e.clientX, start: shown(value), pointer: e.pointerId };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    onbegin?.();
    e.preventDefault();
  }
  function scrubMove(e: PointerEvent) {
    if (!scrub || e.pointerId !== scrub.pointer) return;
    const k = e.shiftKey ? 10 : e.altKey ? 0.1 : 1;
    const steps = Math.round((e.clientX - scrub.x) / 4);
    const next = clampModel(model(roundShown(scrub.start + steps * dstep * k)));
    if (next !== value) onchange(next);
  }
  function scrubEnd(e: PointerEvent) {
    if (!scrub || e.pointerId !== scrub.pointer) return;
    scrub = null;
    onend?.();
  }
</script>

<div class="field" class:disabled>
  <label
    class="label"
    for={uid}
    onpointerdown={scrubStart}
    onpointermove={scrubMove}
    onpointerup={scrubEnd}
    onpointercancel={scrubEnd}
  >
    <span class="ltext">{label}</span>
    {#if hint}
      <span class="hint" use:tip={hint} tabindex="-1"><Info size={11} /></span>
    {/if}
    {#if modified && onreset}
      <button
        class="reset"
        type="button"
        aria-label={resetHint}
        use:tip={resetHint}
        onpointerdown={(e) => e.stopPropagation()}
        onclick={(e) => {
          e.preventDefault();
          onreset();
        }}><RotateCcw size={11} /></button
      >
    {/if}
  </label>
  <input
    id={uid}
    aria-label={label}
    aria-invalid={invalid || undefined}
    class="input tabular"
    class:bad={invalid}
    inputmode="decimal"
    {disabled}
    bind:value={text}
    onfocus={(e) => {
      focused = true;
      (e.currentTarget as HTMLInputElement).select();
    }}
    onblur={() => {
      focused = false;
      commit();
    }}
    onkeydown={key}
  />
  <span class="unit">{shownUnit}</span>
</div>

<style>
  .field {
    display: grid;
    grid-template-columns: 100px minmax(0, 1fr) 30px;
    align-items: center;
    gap: var(--s2);
    min-height: var(--control-h);
  }

  .label {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
    cursor: ew-resize;
    user-select: none;
    touch-action: none;
    min-width: 0;
  }

  .ltext {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .label:hover .ltext {
    color: var(--text-primary);
  }

  .disabled .label {
    cursor: default;
    opacity: 0.5;
  }

  .hint {
    display: inline-flex;
    flex: none;
    color: var(--text-muted);
    cursor: help;
  }

  .hint:hover {
    color: var(--text-primary);
  }

  input {
    width: 100%;
    text-align: right;
  }

  input:disabled {
    opacity: 0.5;
  }

  .bad {
    border-color: var(--danger) !important;
  }

  .reset {
    display: inline-flex;
    flex: none;
    padding: 0;
    border: 0;
    background: none;
    color: var(--text-muted);
  }

  .reset:hover {
    color: var(--text-primary);
  }

  .unit {
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }
</style>
