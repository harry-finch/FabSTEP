<script lang="ts">
  // Menu déroulant ancré à son déclencheur, en position fixe pour ne pas être
  // rogné par les panneaux qui défilent. S'ouvre vers le haut s'il manque de
  // place. Fermeture : clic extérieur, Échap, défilement, ou `close()`.
  import { tick, type Snippet } from 'svelte';

  let {
    trigger,
    content,
    width = 320,
    label,
  }: {
    trigger: Snippet<[{ open: boolean; toggle: () => void }]>;
    content: Snippet<[{ close: () => void }]>;
    width?: number;
    label: string;
  } = $props();

  let open = $state(false);
  let root: HTMLDivElement;
  let panel = $state<HTMLDivElement>();
  let pos = $state({ top: 0, left: 0, maxHeight: 400 });

  async function place() {
    const r = root.getBoundingClientRect();
    const margin = 8;
    const below = window.innerHeight - r.bottom - margin;
    const above = r.top - margin;
    const left = Math.max(margin, Math.min(r.left, window.innerWidth - width - margin));
    await tick();
    const h = Math.min(panel?.scrollHeight ?? 400, 520);
    if (below >= Math.min(h, 260) || below >= above) {
      pos = { top: r.bottom + 4, left, maxHeight: below - 4 };
    } else {
      const maxHeight = above - 4;
      pos = { top: Math.max(margin, r.top - 4 - Math.min(h, maxHeight)), left, maxHeight };
    }
  }

  async function toggle() {
    open = !open;
    if (open) await place();
  }

  const close = () => (open = false);

  function outside(e: PointerEvent) {
    const t = e.target as Node;
    if (open && !root.contains(t) && !panel?.contains(t)) close();
  }

  function key(e: KeyboardEvent) {
    if (open && e.key === 'Escape') {
      close();
      (root.querySelector('button') as HTMLButtonElement | null)?.focus();
    }
  }

  function scrolled(e: Event) {
    if (open && !panel?.contains(e.target as Node)) close();
  }
</script>

<svelte:window onpointerdown={outside} onkeydown={key} onresize={close} onscrollcapture={scrolled} />

<div class="pop" bind:this={root}>
  {@render trigger({ open, toggle })}
</div>
{#if open}
  <div
    class="panel"
    bind:this={panel}
    style:top={`${pos.top}px`}
    style:left={`${pos.left}px`}
    style:width={`${width}px`}
    style:max-height={`${Math.min(pos.maxHeight, 520)}px`}
    role="dialog"
    aria-label={label}
  >
    {@render content({ close })}
  </div>
{/if}

<style>
  .pop {
    width: 100%;
  }

  .panel {
    position: fixed;
    z-index: 500;
    overflow: auto;
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-pop);
    padding: var(--s1);
  }
</style>
