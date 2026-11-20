<script lang="ts">
  import type { ComponentProps } from 'svelte';
  import { t } from '../lib/i18n.svelte';
  import type Viewport from './Viewport.svelte';

  let props: ComponentProps<typeof Viewport> = $props();
  const load = () => import('./Viewport.svelte');
  let chunk = $state.raw<ReturnType<typeof load> | null>(null);
  $effect(() => {
    if (props.bytes && !chunk) chunk = load();
  });
</script>

{#if chunk}
  {#await chunk then module}
    <module.default {...props} />
  {:catch}
    {#if props.bytes}
      <div class="load-error" role="alert">
        <span>{t('viewport.loadFailed')}</span>
        <button class="btn" onclick={() => chunk = load()}>{t('viewport.retry')}</button>
      </div>
    {/if}
  {/await}
{/if}

<style>
  .load-error { position:absolute; inset:0; display:flex; align-items:center; justify-content:center; gap:var(--s2); color:var(--danger-text); font-size:var(--fs-sm); }
</style>
