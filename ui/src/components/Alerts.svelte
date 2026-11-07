<script lang="ts" module>
  export interface Alert {
    /** Code stable du moteur (`plate.margin_too_large`), sert de clé. */
    code: string;
    severity: 'error' | 'warning' | 'info';
    /** Texte déjà traduit. */
    message: string;
  }
</script>

<script lang="ts">
  // Messages du moteur : erreurs et avertissements toujours visibles, les
  // informations repliées derrière un compteur avec un aperçu de la première.
  // Chaque message = icône + texte (jamais la couleur seule).
  import { CircleX, TriangleAlert, Info, ChevronRight } from '@lucide/svelte';
  import { t } from '../lib/i18n.svelte';

  let { alerts }: { alerts: Alert[] } = $props();
  let showInfo = $state(false);

  const strong = $derived(alerts.filter((a) => a.severity !== 'info'));
  const infos = $derived(alerts.filter((a) => a.severity === 'info'));
</script>

{#if alerts.length}
  <div class="alerts" aria-live="polite">
    {#each strong as a (a.code)}
      <div class="row {a.severity}">
        {#if a.severity === 'error'}<CircleX size={14} />{:else}<TriangleAlert size={14} />{/if}
        <span>{a.message}</span>
      </div>
    {/each}
    {#if infos.length}
      <button class="more" type="button" aria-expanded={showInfo} onclick={() => (showInfo = !showInfo)}>
        <ChevronRight size={12} class={showInfo ? 'open' : ''} />
        <Info size={13} />
        <span class="count">{t(infos.length > 1 ? 'alerts.infos' : 'alerts.info', { n: infos.length })}</span>
        {#if !showInfo}<span class="peek">{infos[0].message}</span>{/if}
      </button>
      {#if showInfo}
        {#each infos as a (a.code)}
          <div class="row info"><Info size={13} /><span>{a.message}</span></div>
        {/each}
      {/if}
    {/if}
  </div>
{/if}

<style>
  .alerts {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .row {
    display: flex;
    align-items: flex-start;
    gap: var(--s2);
    padding: 5px var(--s2);
    border-radius: var(--radius);
    font-size: var(--fs-sm);
    line-height: 1.45;
  }

  .row :global(svg) {
    flex: none;
    margin-top: 1px;
  }

  /* Seule l'erreur bloquante reçoit un fond teinté. */
  .row.error {
    background: var(--danger-tint);
    color: var(--danger-text);
  }

  .row.error :global(svg) {
    color: var(--danger);
  }

  .row.warning {
    color: var(--text-primary);
  }

  .row.warning :global(svg) {
    color: var(--warning);
  }

  .row.info {
    color: var(--text-secondary);
  }

  .row.info :global(svg) {
    color: var(--info);
  }

  .more {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    align-self: flex-start;
    max-width: 100%;
    min-width: 0;
    padding: 3px var(--s2);
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }

  .more:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .more :global(.open) {
    transform: rotate(90deg);
  }

  .count {
    flex: none;
    font-weight: 600;
  }

  .peek {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .peek::before {
    content: '— ';
  }
</style>
