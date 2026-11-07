<script lang="ts">
  // Ligne de divulgation discrète (« ▸ Avancé ») ; l'état est mémorisé.
  import { ChevronRight } from '@lucide/svelte';
  import type { Snippet } from 'svelte';
  import { storageKey } from '../lib/app';

  let { id, label, children }: { id: string; label: string; children: Snippet } = $props();

  const key = () => storageKey(`disclosure.${id}`);
  function initial(): boolean {
    try {
      return localStorage.getItem(key()) === '1';
    } catch {
      return false;
    }
  }
  let open = $state(initial());

  function toggle() {
    open = !open;
    try {
      localStorage.setItem(key(), open ? '1' : '0');
    } catch {
      /* ignoré */
    }
  }
</script>

<button class="disc" class:open type="button" aria-expanded={open} onclick={toggle}>
  <ChevronRight size={13} class="chev" />
  {label}
</button>
{#if open}
  <div class="body">{@render children()}</div>
{/if}

<style>
  .disc {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 24px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }

  .disc:hover {
    color: var(--text-primary);
  }

  .disc :global(.chev) {
    transition: transform 120ms;
  }

  .open :global(.chev) {
    transform: rotate(90deg);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }
</style>
