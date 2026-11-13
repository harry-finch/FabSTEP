<script lang="ts">
  // Étapes asynchrones : états et progression fournis par l'application.
  type Task = { id: string; title: string; state: string; detail?: string; progress?: number; cancelLabel?: string; oncancel?: () => void };
  let { tasks, label }: { tasks: Task[]; label: string } = $props();
</script>
<ul class="tasks" aria-label={label}>
  {#each tasks as task (task.id)}
    <li>
      <div class="row"><strong>{task.title}</strong><span class="state">{task.state}</span></div>
      {#if task.detail}<p>{task.detail}</p>{/if}
      {#if task.progress != null && Number.isFinite(task.progress)}
        <progress value={Math.max(0, Math.min(100, task.progress))} max="100" aria-label={task.title}></progress>
      {/if}
      {#if task.oncancel && task.cancelLabel}<button class="btn quiet" onclick={task.oncancel}>{task.cancelLabel}</button>{/if}
    </li>
  {/each}
</ul>
<style>
  .tasks { list-style: none; padding: 0; margin: 0; }
  li { padding: var(--s2) 0; border-bottom: 1px solid var(--divider); }
  .row { display: flex; justify-content: space-between; gap: var(--s2); }
  strong { font-weight: 500; } .state, p { color: var(--text-secondary); font-size: var(--fs-sm); }
  p { margin: var(--s1) 0; } progress { width: 100%; height: 4px; accent-color: var(--accent); }
</style>
