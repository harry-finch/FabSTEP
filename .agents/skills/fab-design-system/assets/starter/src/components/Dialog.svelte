<script lang="ts">
  // Fenêtre modale native (<dialog>) : en-tête titre + fermer, corps qui
  // défile, pied d'actions facultatif (secondaire à gauche, principal à
  // droite). Échap et le bouton fermer appellent `onclose`. Réservée aux
  // tâches qui interrompent vraiment (aide, saisie d'un essai, import) :
  // jamais pour une action courante.
  import { X } from '@lucide/svelte';
  import type { Snippet } from 'svelte';
  import { t } from '../lib/i18n.svelte';

  let {
    title,
    width = 560,
    onclose,
    children,
    footer,
  }: {
    title: string;
    width?: number;
    onclose: () => void;
    children: Snippet;
    footer?: Snippet<[{ close: () => void }]>;
  } = $props();

  let dialog = $state<HTMLDialogElement>();
  const uid = `dlg-${Math.random().toString(36).slice(2, 9)}`;
  const close = () => dialog?.close();

  $effect(() => dialog?.showModal());
</script>

<dialog bind:this={dialog} style:width={`min(${width}px, calc(100vw - 32px))`} aria-labelledby={uid} {onclose}>
  <header>
    <h2 id={uid}>{title}</h2>
    <button class="btn quiet icon" type="button" aria-label={t('common.close')} onclick={close}><X size={16} /></button>
  </header>
  <div class="body">{@render children()}</div>
  {#if footer}
    <footer>{@render footer({ close })}</footer>
  {/if}
</dialog>

<style>
  dialog {
    padding: 0;
    border: 0;
    border-radius: var(--radius-lg);
    background: var(--bg-elevated);
    color: var(--text-primary);
    box-shadow: var(--shadow-pop);
  }

  dialog::backdrop {
    background: var(--backdrop);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--s3) var(--s4);
    border-bottom: 1px solid var(--border);
  }

  h2 {
    margin: 0;
    font-size: var(--fs-lg);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    padding: var(--s3) var(--s4) var(--s4);
    font-size: var(--fs-md);
    line-height: 1.55;
    max-height: 70vh;
    overflow: auto;
  }

  /* Sous-titres d'une fenêtre : petites capitales, comme les sections. */
  .body :global(h3) {
    margin: var(--s3) 0 0;
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-section);
  }

  footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--s2);
    padding: var(--s3) var(--s4);
    border-top: 1px solid var(--border);
  }
</style>
