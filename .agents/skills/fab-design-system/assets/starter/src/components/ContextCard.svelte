<script lang="ts">
  // Contexte et provenance : l'application fournit une URL uniquement si elle est connue.
  let { title, excerpt, source, href }: { title: string; excerpt: string; source: string; href?: string } = $props();
</script>
<article class="context">
  <strong>{title}</strong><p>{excerpt}</p>
  {#if href}<a href={href}>{source}</a>{:else}<span>{source}</span>{/if}
</article>
<style>
  .context { padding: var(--s2); border: 1px solid var(--border); border-radius: var(--radius); background: var(--bg-panel); }
  strong { font-weight: 600; } p { margin: var(--s1) 0; } span, a { font-size: var(--fs-sm); color: var(--text-secondary); }
  a { color: var(--accent-text); overflow-wrap: anywhere; }
</style>
