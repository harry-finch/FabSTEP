<script lang="ts">
  // État flottant non modal : phase réelle et durée écoulée, sans progression chiffrée.
  import { t } from '../lib/i18n.svelte';

  let { phase, startedAt }: { phase: 'queued' | 'preview' | 'converting'; startedAt: number } = $props();
  let elapsed = $state(0);
  const clock = $derived(`${String(Math.floor(elapsed / 60)).padStart(2, '0')}:${String(elapsed % 60).padStart(2, '0')}`);

  $effect(() => {
    const start = startedAt;
    const update = () => { elapsed = Math.floor(Math.max(0, performance.now() - start) / 1000); };
    update();
    const timer = setInterval(update, 250);
    return () => clearInterval(timer);
  });
</script>

<div class="processing">
  <div class="heading">
    <svg class="glyph" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 2.5 20.2 7.2v9.6L12 21.5l-8.2-4.7V7.2L12 2.5Z" />
      <path d="m3.8 7.2 8.2 4.8 8.2-4.8M12 12v9.5" />
    </svg>
    <div class="copy" role="status" aria-live="polite" aria-atomic="true">
      <strong>{t(`processing.${phase}Title`)}</strong>
      <span>{t(`processing.${phase}Detail`)}</span>
    </div>
    <span class="elapsed" aria-hidden="true">{clock}</span>
  </div>
  <div class="track" aria-hidden="true"><div class="activity"></div></div>
</div>

<style>
  .processing {
    position: absolute;
    top: var(--s4);
    right: var(--s4);
    width: min(280px, calc(100% - 2 * var(--s4)));
    padding: var(--s3);
    background: var(--bg-panel);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    pointer-events: none;
  }
  .heading { display: flex; align-items: flex-start; gap: var(--s2); }
  .glyph { width: 24px; height: 24px; flex: none; fill: none; stroke: var(--accent); stroke-width: 1.25; stroke-linejoin: round; animation: breathe 2s ease-in-out infinite; }
  .copy { min-width: 0; flex: 1; display: flex; flex-direction: column; gap: var(--s1); }
  .copy strong { color: var(--text-primary); font-size: var(--fs-md); font-weight: 600; }
  .copy span { color: var(--text-secondary); font-size: var(--fs-sm); line-height: 1.4; }
  .elapsed { color: var(--text-muted); font-size: var(--fs-xs); font-variant-numeric: tabular-nums; }
  .track { height: var(--s1); margin-top: var(--s3); overflow: hidden; background: var(--track); border-radius: var(--radius); }
  .activity { width: 35%; height: 100%; background: var(--accent); border-radius: inherit; animation: travel 1.6s ease-in-out infinite; }
  @keyframes travel { from { transform: translateX(-100%); } to { transform: translateX(285%); } }
  @keyframes breathe { 50% { opacity: .55; } }
  @media (prefers-reduced-motion: reduce) { .glyph, .activity { animation: none; } }
</style>
