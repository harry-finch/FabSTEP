<script lang="ts">
  // Barre d'état : pastille d'état du moteur (ou de la connexion), versions,
  // puis à droite le contexte (position du curseur, machine active…).
  // Toujours discrète : 24 px, texte muted, séparateurs fins.
  import type { Snippet } from 'svelte';
  import { t } from '../lib/i18n.svelte';

  let {
    state = 'ready',
    version,
    children,
    right,
  }: {
    state?: 'loading' | 'ready' | 'failed';
    version?: string;
    /** Éléments à gauche, après la version (chacun précédé de <span class="sep">). */
    children?: Snippet;
    right?: Snippet;
  } = $props();
</script>

<footer class="status">
  <span class="dot" class:off={state === 'failed'} class:wait={state === 'loading'}></span>
  <span>{t(`status.${state}`)}</span>
  {#if version}
    <span class="sep"></span>
    <span>{t('status.engine', { v: version })}</span>
  {/if}
  {@render children?.()}
  <span class="grow"></span>
  {@render right?.()}
</footer>

<style>
  .status {
    display: flex;
    align-items: center;
    gap: var(--s2);
    height: 24px;
    padding: 0 var(--s3);
    background: var(--bg-chrome);
    border-top: 1px solid var(--border);
    color: var(--text-muted);
    font-size: var(--fs-xs);
    white-space: nowrap;
    overflow: hidden;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--success);
  }

  .dot.wait {
    background: var(--warning);
  }

  .dot.off {
    background: var(--danger);
  }

  .status :global(.sep) {
    width: 1px;
    height: 12px;
    background: var(--border-strong);
  }

  .grow {
    flex: 1;
  }
</style>
