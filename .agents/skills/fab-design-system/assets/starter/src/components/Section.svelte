<script lang="ts">
  // Section repliable d'un panneau : titre discret en petites capitales,
  // numéro d'étape optionnel, actions à droite. L'état ouvert est mémorisé.
  import { ChevronRight } from '@lucide/svelte';
  import type { Snippet } from 'svelte';
  import { storageKey } from '../lib/app';

  let {
    id,
    title,
    step,
    open: initiallyOpen = true,
    actions,
    children,
  }: {
    id: string;
    title: string;
    step?: number;
    open?: boolean;
    actions?: Snippet;
    children: Snippet;
  } = $props();

  const key = () => storageKey(`section.${id}`);
  function initial(): boolean {
    try {
      const v = localStorage.getItem(key());
      return v == null ? initiallyOpen : v === '1';
    } catch {
      return initiallyOpen;
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

<section class="section" class:open>
  <header>
    <button class="toggle" type="button" aria-expanded={open} onclick={toggle}>
      <ChevronRight size={14} class="chev" />
      {#if step}<span class="step">{step}</span>{/if}
      <span class="title">{title}</span>
    </button>
    {#if actions}<div class="actions">{@render actions()}</div>{/if}
  </header>
  {#if open}
    <div class="body">{@render children()}</div>
  {/if}
</section>

<style>
  .section {
    border-bottom: 1px solid var(--divider);
  }

  header {
    display: flex;
    align-items: center;
    height: 34px;
    padding-right: var(--s2);
  }

  .toggle {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 100%;
    padding: 0 var(--s3);
    background: none;
    border: 0;
    text-align: left;
    color: var(--text-section);
  }

  .toggle:hover {
    color: var(--text-primary);
  }

  .toggle :global(.chev) {
    transition: transform 120ms;
    flex: none;
    color: var(--text-muted);
  }

  .open .toggle :global(.chev) {
    transform: rotate(90deg);
  }

  .step {
    display: inline-grid;
    place-items: center;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1px solid var(--border-strong);
    font-size: 9.5px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .title {
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .actions {
    display: flex;
    gap: 2px;
  }

  .body {
    padding: 2px var(--s3) var(--s3);
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }
</style>
