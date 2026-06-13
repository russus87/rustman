<script>
  // Visualizza un diff (unificato) come contenuto di un tab.
  let { titolo, righe = [] } = $props();
</script>

<div class="diff-wrap">
  <div class="diff-titolo">{titolo}</div>
  {#if righe.length === 0}
    <div class="placeholder"><div>Nessuna differenza.</div></div>
  {:else}
    <div class="diff-corpo">
      {#each righe as r}
        <div class="drow" class:add={r.tipo === "add"} class:rem={r.tipo === "rem"}>
          <span class="ln">{r.nuova ?? r.vecchia ?? ""}</span>
          <span class="ct">{r.testo}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .diff-wrap { flex: 1; display: flex; flex-direction: column; min-height: 0; }
  .diff-titolo { padding: 10px 16px; border-bottom: 1px solid var(--border); font-family: var(--mono); font-size: 12.5px; color: var(--txt); }
  .diff-corpo { flex: 1; overflow: auto; font-family: var(--mono); font-size: 12.5px; padding: 8px 0; }
</style>
