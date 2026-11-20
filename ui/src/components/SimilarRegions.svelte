<script lang="ts">
  import { onDestroy } from 'svelte';
  import { i18n, t } from '../lib/i18n.svelte';
  import type { SourceBridge } from '../lib/source';
  import type { ResultFaces } from '../lib/types';
  import { nextSimilarSearch, previewSimilar, type SimilarRegions } from '../lib/similar-regions';
  let {doc, bridge, onpreview, onapply}:{
    doc:ResultFaces; bridge:SourceBridge; onpreview:(triangles:number[]|null)=>void; onapply:(ids:number[], searchId:number)=>void
  } = $props();
  let found = $state.raw<SimilarRegions|null>(null);
  let chosen = $state.raw<number[]>([]);
  let pending = $state(false), failed = $state(false);
  let epoch = 0;
  let ticket:number|null = null;
  function cancel() {
    if (ticket !== null) void bridge.invoke('cancel_similar_search', {requestId:doc.request_id,sourceId:doc.source_id,searchId:ticket}).catch(()=>{});
    ticket = null;
    epoch++; pending = false; failed = false; found = null; chosen = []; onpreview(null);
  }
  onDestroy(cancel);
  async function search() {
    const job = ++epoch, searchId = nextSimilarSearch();
    ticket = searchId;
    pending = true; failed = false;
    try {
      const reply = await bridge.invoke<{request_id:number; source_id:number; search_id:number; regions:SimilarRegions}>(
        'find_similar_guided_regions',{requestId:doc.request_id,sourceId:doc.source_id,searchId});
      if (job !== epoch) return;
      if (reply.request_id !== doc.request_id || reply.source_id !== doc.source_id || reply.search_id !== searchId) throw new Error('engine.stale');
      found = reply.regions;
      select(found.candidates.map(candidate => candidate.id));
    } catch {
      if (job === epoch) failed = true;
    } finally {
      if (job === epoch) pending = false;
    }
  }
  function select(ids:number[]) {
    chosen = ids;
    if (found) onpreview(chosen.length ? previewSimilar(doc, found, chosen) : null);
  }
  function apply() {
    if (ticket === null) return;
    const ids = chosen, searchId = ticket;
    ticket = null; cancel(); onapply(ids, searchId);
  }
  const pattern = $derived(found?.candidates.find(candidate => candidate.pattern)?.pattern);
  const members = $derived(found?.candidates.filter(candidate => candidate.pattern && candidate.pattern === pattern).map(candidate => candidate.id) ?? []);
  const allMembers = $derived(members.every(id => chosen.includes(id)));
  function togglePattern() {
    const others = chosen.filter(id => !members.includes(id));
    select(allMembers ? others : [...others, ...members]);
  }
  const fmt = $derived(new Intl.NumberFormat(i18n.locale,{maximumSignificantDigits:4}));
</script>

{#if !found && !pending}
  <button class="btn" onclick={search}>{t('similar.search')}</button>
  {#if failed}<p role="status">{t('similar.failed')}</p>{/if}
{:else}
  {#if pending}<p role="status">{t('similar.pending')}</p>{/if}
  {#if found}
    <p role="status">{t('similar.count',{n:found.candidates.length})}</p>
    {#if found.truncated}<p>{t('similar.limit')}</p>{/if}
    {#if pattern}
      <label>
        <input type="checkbox" checked={allMembers} indeterminate={!allMembers && members.some(id => chosen.includes(id))} onchange={togglePattern}/>
        {t(`similar.${pattern}`,{n:members.length})}
      </label>
    {/if}
    {#each found.candidates as candidate (candidate.id)}
      <label>
        <input type="checkbox" checked={chosen.includes(candidate.id)}
          onchange={(event)=>select(event.currentTarget.checked ? [...chosen,candidate.id] : chosen.filter(id=>id!==candidate.id))}/>
        {t('similar.wall',{n:candidate.face_ids[0]+1})}
        {#if candidate.radius_mm != null} · R {fmt.format(candidate.radius_mm)} mm{/if}
        {#if candidate.semi_angle_rad != null} · {fmt.format(candidate.semi_angle_rad * 180 / Math.PI)}°{/if}
      </label>
    {/each}
  {/if}
  <div class="actions">
    {#if found?.candidates.length}<button class="btn" disabled={!chosen.length} onclick={apply}>{t('similar.apply')}</button>{/if}
    <button class="btn quiet" onclick={cancel}>{t('similar.cancel')}</button>
  </div>
{/if}

<style>
  p,label{margin:0;color:var(--text-secondary);font-size:var(--fs-xs)}
  label,.actions{display:flex;align-items:center;gap:var(--s1);min-height:var(--control-h)}
  input{accent-color:var(--accent)}
</style>
