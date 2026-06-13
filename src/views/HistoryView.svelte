<script>
  // Vista "History": cronologia delle richieste inviate, con replay (riapertura
  // della richiesta) e confronto (diff) di due risposte.
  let { storia = [], onApri, onPulisci, onAggiorna, onConfronta } = $props();

  // Indici selezionati per il confronto (massimo 2).
  let sel = $state([]);
  function toggle(i) {
    if (sel.includes(i)) sel = sel.filter((x) => x !== i);
    else if (sel.length < 2) sel = [...sel, i];
  }
  function confronta() {
    if (sel.length === 2) {
      // Confronta dal più vecchio (indice maggiore) al più recente (minore).
      const [a, b] = [...sel].sort((x, y) => y - x);
      onConfronta?.(storia[a], storia[b]);
    }
  }

  function classeMetodo(m) {
    if (m === "GET") return "get";
    if (m === "POST") return "post";
    if (m === "PUT" || m === "PATCH") return "put";
    if (m === "DELETE") return "del";
    return "";
  }
  function quando(iso) {
    try {
      return new Date(iso).toLocaleString();
    } catch {
      return iso;
    }
  }
  function classeStatus(s) {
    return s >= 400 || s === 0 ? "ko" : "ok";
  }
</script>

<div class="sto-head">
  HISTORY
  <span class="act" title="Aggiorna" onclick={onAggiorna}>⟳</span>
  <span class="act" title="Svuota cronologia" onclick={onPulisci}>🗑</span>
</div>

{#if sel.length === 2}
  <div class="cmp-bar">
    <button class="cmp-btn" onclick={confronta}>⇄ Confronta le 2 risposte</button>
    <span class="cmp-x" title="Annulla" onclick={() => (sel = [])}>✕</span>
  </div>
{:else if storia.length > 1}
  <div class="cmp-hint">Spunta due voci per confrontarne le risposte.</div>
{/if}

<div class="sto-list">
  {#each storia as v, i}
    <div class="sto-item" class:sel={sel.includes(i)}>
      <input type="checkbox" class="cmp-cb" checked={sel.includes(i)} onclick={() => toggle(i)} title="Seleziona per il confronto" />
      <div class="sto-body" onclick={() => onApri(v)} title="Apri per reinviare (replay)">
        <div class="riga1">
          <span class="m {classeMetodo(v.richiesta.metodo)}">{v.richiesta.metodo}</span>
          <span class="st {classeStatus(v.status)}">{v.status || "ERR"}</span>
          <span class="tempo">{v.tempo_ms} ms</span>
        </div>
        <div class="url" title={v.richiesta.url}>{v.richiesta.url}</div>
        <div class="meta">
          {quando(v.quando)}{v.ambiente ? ` · ${v.ambiente}` : ""}
        </div>
      </div>
    </div>
  {/each}
  {#if storia.length === 0}
    <div class="vuoto">Nessuna richiesta inviata.</div>
  {/if}
</div>

<style>
  .sto-head { padding: 14px 14px 8px; font-size: 11px; font-weight: 600; letter-spacing: .8px; color: var(--txt-faint); display: flex; align-items: center; gap: 6px; }
  .sto-head .act { margin-left: auto; cursor: pointer; color: var(--txt-dim); padding: 0 3px; border-radius: 4px; }
  .sto-head .act:last-child { margin-left: 4px; }
  .sto-head .act:hover { background: var(--panel-3); color: var(--txt); }
  .cmp-bar { display: flex; align-items: center; gap: 8px; padding: 6px 12px; }
  .cmp-btn { flex: 1; background: linear-gradient(145deg,#8b6dff,#6c47ff); border: none; color: #fff; border-radius: 6px; padding: 6px 10px; font-size: 12px; cursor: pointer; }
  .cmp-x { cursor: pointer; color: var(--txt-faint); padding: 0 4px; }
  .cmp-x:hover { color: var(--red); }
  .cmp-hint { padding: 4px 12px 8px; font-size: 11px; color: var(--txt-faint); }
  .sto-list { overflow-y: auto; padding: 4px 8px; }
  .sto-item { display: flex; align-items: flex-start; gap: 8px; padding: 8px; border-radius: 6px; border: 1px solid transparent; }
  .sto-item:hover { background: var(--panel-3); border-color: var(--border); }
  .sto-item.sel { border-color: var(--accent); background: rgba(124,92,255,.10); }
  .cmp-cb { margin-top: 3px; cursor: pointer; }
  .sto-body { flex: 1; cursor: pointer; min-width: 0; }
  .riga1 { display: flex; align-items: center; gap: 8px; margin-bottom: 3px; }
  .m { font-family: var(--mono); font-size: 10.5px; font-weight: 700; }
  .m.get { color: var(--green); }
  .m.post { color: #e2b340; }
  .m.put { color: #4a9eff; }
  .m.del { color: var(--red); }
  .st { font-size: 11px; font-weight: 700; margin-left: auto; }
  .st.ok { color: var(--green); }
  .st.ko { color: var(--red); }
  .tempo { font-size: 10.5px; color: var(--txt-faint); }
  .url { font-size: 12px; color: var(--txt-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .meta { font-size: 10.5px; color: var(--txt-faint); margin-top: 2px; }
  .vuoto { color: var(--txt-faint); padding: 10px 8px; font-size: 12px; }
</style>
