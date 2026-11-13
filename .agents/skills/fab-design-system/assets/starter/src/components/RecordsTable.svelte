<script lang="ts">
  // Tableau filtrable : chaînes et données fournies par l'application.
  type Row = { id: string; cells: string[]; status: string };
  let { columns, rows, searchLabel, filterLabel, allLabel, emptyLabel, noResultsLabel, countLabel }: { columns: string[]; rows: Row[]; searchLabel: string; filterLabel: string; allLabel: string; emptyLabel: string; noResultsLabel: string; countLabel: string } = $props();
  let query = $state(''); let filter = $state('');
  let statuses = $derived([...new Set(rows.map((row) => row.status))]);
  let visible = $derived(rows.filter((row) => (!filter || row.status === filter) && row.cells.some((cell) => cell.toLocaleLowerCase().includes(query.toLocaleLowerCase()))));
</script>
<div class="table-wrap">
  <div class="filters">
    <label>{searchLabel}<input class="input" type="search" bind:value={query} /></label>
    <label>{filterLabel}<select class="input" bind:value={filter}><option value="">{allLabel}</option>{#each statuses as status}<option value={status}>{status}</option>{/each}</select></label>
    <span class="count">{visible.length} {countLabel}</span>
  </div>
  {#if rows.length === 0}<p>{emptyLabel}</p>{:else if visible.length === 0}<p>{noResultsLabel}</p>{:else}
    <div class="scroll"><table><thead><tr>{#each columns as column}<th scope="col">{column}</th>{/each}</tr></thead><tbody>{#each visible as row (row.id)}<tr>{#each columns as _, i}<td>{row.cells[i] ?? ''}</td>{/each}</tr>{/each}</tbody></table></div>
  {/if}
</div>
<style>
  .filters { display: flex; flex-wrap: wrap; align-items: end; gap: var(--s2); }
  label { display: grid; gap: var(--s1); color: var(--text-secondary); font-size: var(--fs-sm); }
  .count, p { color: var(--text-secondary); font-size: var(--fs-sm); } .count { padding-bottom: var(--s1); }
  .scroll { max-width: 100%; overflow-x: auto; } table { border-collapse: collapse; width: 100%; margin-top: var(--s2); text-align: left; }
  th, td { padding: var(--s1) var(--s2); border-bottom: 1px solid var(--divider); } th { color: var(--text-secondary); font-weight: 500; }
</style>
