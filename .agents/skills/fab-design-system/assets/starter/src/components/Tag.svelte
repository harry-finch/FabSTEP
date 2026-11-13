<script lang="ts">
  // Petite étiquette d'état ou d'origine d'une valeur : icône + texte sur un
  // fond très légèrement teinté (jamais la couleur seule). Le texte peut être
  // masqué quand la place manque (`compact`) : il reste dans l'infobulle.
  // Tons : neutral (calculé), info (donnée de référence), success (validé),
  // warning (adapté, dérivé), danger (refusé), accent (choix de l'utilisateur).
  import type { Component } from 'svelte';
  import { tip } from '../lib/tip';

  let {
    label,
    hint,
    icon: Icon,
    tone = 'neutral',
    compact = false,
  }: {
    label: string;
    hint?: string;
    icon?: Component<{ size?: number }>;
    tone?: 'neutral' | 'info' | 'success' | 'warning' | 'danger' | 'accent';
    compact?: boolean;
  } = $props();
</script>

<span class="tag {tone}" aria-label={label} use:tip={hint ?? (compact ? label : undefined)}>
  {#if Icon}<Icon size={11} />{/if}
  {#if !compact}<span class="text">{label}</span>{/if}
</span>

<style>
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 1px 5px;
    border-radius: 3px;
    font-size: var(--fs-xs);
    font-weight: 600;
    white-space: nowrap;
  }

  .neutral {
    color: var(--text-secondary);
    background: var(--neutral-tint);
  }

  .info {
    color: var(--info);
    background: var(--info-tint);
  }

  .success {
    color: var(--success);
    background: var(--success-tint);
  }

  .warning {
    color: var(--warning);
    background: var(--warning-tint);
  }

  .danger {
    color: var(--danger);
    background: var(--danger-tint);
  }

  .accent {
    color: var(--accent-text);
    background: var(--accent-tint);
  }
</style>
