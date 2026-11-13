<script lang="ts">
  // Panneau facultatif : messages et actions contrôlés par l'application hôte.
  type Message = { id: string; author: string; text: string };
  let { label, messages, inputLabel, sendLabel, busyLabel, busy = false, onsubmit }: { label: string; messages: Message[]; inputLabel: string; sendLabel: string; busyLabel: string; busy?: boolean; onsubmit: (text: string) => void } = $props();
  const inputId = $props.id();
  let text = $state('');
  function submit(event: SubmitEvent) { event.preventDefault(); const value = text.trim(); if (!value || busy) return; onsubmit(value); text = ''; }
</script>
<section class="assistant" aria-label={label}>
  <div class="messages" aria-live="polite" aria-relevant="additions text">
    {#each messages as message (message.id)}<div class="message"><strong>{message.author}</strong><p>{message.text}</p></div>{/each}
    {#if busy}<p class="busy">{busyLabel}</p>{/if}
  </div>
  <form onsubmit={submit}><label for={inputId}>{inputLabel}</label><textarea id={inputId} class="input" bind:value={text} rows="2"></textarea><button class="btn" disabled={busy || !text.trim()}>{sendLabel}</button></form>
</section>
<style>
  .assistant { display: grid; gap: var(--s2); } .messages { display: grid; gap: var(--s2); }
  .message { border-left: 2px solid var(--border-strong); padding-left: var(--s2); } strong { font-weight: 600; }
  p { margin: var(--s1) 0 0; white-space: pre-wrap; overflow-wrap: anywhere; } .busy, label { color: var(--text-secondary); font-size: var(--fs-sm); }
  form { display: grid; gap: var(--s1); } textarea { height: auto; min-height: 52px; padding: var(--s2); resize: vertical; } button { justify-self: end; }
</style>
