<script lang="ts">
  // Proposition explicite : aucune mutation avant l'action de l'utilisateur.
  let { title, description, applyLabel, rejectLabel, onapply, onreject }: { title: string; description: string; applyLabel: string; rejectLabel: string; onapply: () => void; onreject: () => void } = $props();
</script>
<article class="proposal">
  <strong>{title}</strong><p>{description}</p>
  <div class="actions"><button class="btn quiet" onclick={onreject}>{rejectLabel}</button><button class="btn primary" onclick={onapply}>{applyLabel}</button></div>
</article>
<style>
  .proposal { padding: var(--s2); border: 1px solid var(--border); border-radius: var(--radius); background: var(--bg-panel); }
  strong { font-weight: 600; } p { margin: var(--s1) 0 var(--s2); color: var(--text-secondary); }
  .actions { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: var(--s1); }
</style>
