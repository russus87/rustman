<script>
  // Palette dei comandi (Ctrl/Cmd+K): filtro fuzzy + navigazione da tastiera.
  let { comandi = [], onChiudi } = $props();

  let query = $state("");
  let attivo = $state(0);
  let inputEl = $state(null);

  // Filtro "fuzzy" semplice: tutti i caratteri della query, in ordine, nel testo.
  function combacia(testo, q) {
    if (!q) return true;
    const t = testo.toLowerCase();
    let i = 0;
    for (const c of q.toLowerCase()) {
      i = t.indexOf(c, i);
      if (i < 0) return false;
      i++;
    }
    return true;
  }
  const filtrati = $derived(
    comandi.filter((c) => combacia(`${c.label} ${c.hint ?? ""}`, query)).slice(0, 50)
  );

  $effect(() => {
    // Resetta la selezione quando cambia la query.
    query;
    attivo = 0;
  });
  $effect(() => {
    inputEl?.focus();
  });

  function esegui(c) {
    onChiudi();
    c?.azione?.();
  }
  function suTasto(e) {
    if (e.key === "Escape") { e.preventDefault(); onChiudi(); }
    else if (e.key === "ArrowDown") { e.preventDefault(); attivo = Math.min(attivo + 1, filtrati.length - 1); }
    else if (e.key === "ArrowUp") { e.preventDefault(); attivo = Math.max(attivo - 1, 0); }
    else if (e.key === "Enter") { e.preventDefault(); esegui(filtrati[attivo]); }
  }
</script>

<div class="overlay" onclick={onChiudi}>
  <div class="palette" onclick={(e) => e.stopPropagation()}>
    <input
      bind:this={inputEl}
      bind:value={query}
      onkeydown={suTasto}
      class="cp-input"
      placeholder="Cerca un comando, una richiesta, un ambiente…"
      spellcheck="false"
    />
    <div class="cp-list">
      {#each filtrati as c, i}
        <div class="cp-item" class:active={i === attivo} onmouseenter={() => (attivo = i)} onclick={() => esegui(c)}>
          {#if c.tag}<span class="cp-tag {c.tagClasse ?? ''}">{c.tag}</span>{/if}
          <span class="cp-label">{c.label}</span>
          {#if c.hint}<span class="cp-hint">{c.hint}</span>{/if}
        </div>
      {/each}
      {#if filtrati.length === 0}
        <div class="cp-vuoto">Nessun comando.</div>
      {/if}
    </div>
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,.45); display: flex; align-items: flex-start; justify-content: center; padding-top: 12vh; z-index: 1000; }
  .palette { width: 560px; max-width: 92vw; background: var(--panel); border: 1px solid var(--border-2); border-radius: 12px; box-shadow: 0 24px 60px rgba(0,0,0,.5); overflow: hidden; }
  .cp-input { width: 100%; background: var(--panel-2); border: none; border-bottom: 1px solid var(--border); padding: 14px 16px; color: var(--txt); font-size: 15px; outline: none; }
  .cp-list { max-height: 50vh; overflow-y: auto; padding: 6px; }
  .cp-item { display: flex; align-items: center; gap: 10px; padding: 9px 10px; border-radius: 7px; cursor: pointer; }
  .cp-item.active { background: rgba(124,92,255,.18); }
  .cp-tag { font-family: var(--mono); font-size: 10px; font-weight: 700; padding: 2px 6px; border-radius: 5px; background: var(--panel-3); color: var(--txt-dim); }
  .cp-tag.get { color: var(--green); } .cp-tag.post { color: #e2b340; }
  .cp-tag.put { color: #4a9eff; } .cp-tag.del { color: var(--red); }
  .cp-label { color: var(--txt); font-size: 13.5px; }
  .cp-hint { margin-left: auto; color: var(--txt-faint); font-size: 11.5px; }
  .cp-vuoto { padding: 14px; color: var(--txt-faint); font-size: 13px; text-align: center; }
</style>
