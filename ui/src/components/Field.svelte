<script lang="ts">
  // Ligne de propriété non numérique : LIBELLÉ | CONTRÔLE, alignée sur NumField.
  import { Info } from '@lucide/svelte';
  import type { Snippet } from 'svelte';
  import { tip } from '../lib/tip';

  let { label, hint, children }: { label: string; hint?: string; children: Snippet } = $props();
</script>

<div class="field">
  <span class="label">
    {label}
    {#if hint}
      <span class="hint" use:tip={hint} tabindex="-1"><Info size={11} /></span>
    {/if}
  </span>
  <div class="value">{@render children()}</div>
</div>

<style>
  .field {
    display: grid;
    grid-template-columns: 100px minmax(0, 1fr);
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
  }

  .hint {
    display: inline-flex;
    color: var(--text-muted);
    cursor: help;
  }

  .hint:hover {
    color: var(--text-primary);
  }

  .value {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--s1);
  }
</style>
