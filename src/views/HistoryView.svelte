<script>
  // Vista "History": cronologia delle richieste inviate, con replay (riapertura
  // della richiesta) e confronto (diff) di due risposte.
  let { storia = [], onApri, onPulisci, onAggiorna, onConfronta } = $props();

  let modo = $state("lista"); // lista | andamento

  // Statistiche dei tempi raggruppate per endpoint (metodo + url).
  const gruppi = $derived.by(() => {
    const m = new Map();
    for (const v of storia) {
      const k = `${v.richiesta.metodo} ${v.richiesta.url}`;
      if (!m.has(k)) m.set(k, []);
      m.get(k).push(v.tempo_ms);
    }
    return [...m.entries()]
      .map(([k, t]) => {
        const ord = [...t].sort((a, b) => a - b);
        const avg = Math.round(t.reduce((a, b) => a + b, 0) / t.length);
        const p95 = ord[Math.min(ord.length - 1, Math.floor(ord.length * 0.95))];
        return { k, n: t.length, avg, p95, max: ord[ord.length - 1], serie: t.slice(0, 30).reverse() };
      })
      .sort((a, b) => b.n - a.n);
  });
  // Punti SVG normalizzati per la sparkline.
  function sparkline(serie, max) {
    if (serie.length < 2) return "";
    const w = 90, h = 22, m = Math.max(max, 1);
    return serie
      .map((v, i) => `${(i / (serie.length - 1)) * w},${h - (v / m) * h}`)
      .join(" ");
  }

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
  <span class="seg" class:on={modo === "lista"} onclick={() => (modo = "lista")}>Lista</span>
  <span class="seg" class:on={modo === "andamento"} onclick={() => (modo = "andamento")}>Andamento</span>
  <span class="act" title="Aggiorna" onclick={onAggiorna}>⟳</span>
  <span class="act" title="Svuota cronologia" onclick={onPulisci}>🗑</span>
</div>

{#if modo === "andamento"}
  <div class="sto-list">
    {#each gruppi as g}
      <div class="trend">
        <div class="t-url" title={g.k}>{g.k}</div>
        <div class="t-riga">
          <svg class="spark" viewBox="0 0 90 22" preserveAspectRatio="none">
            <polyline points={sparkline(g.serie, g.max)} fill="none" stroke="var(--accent)" stroke-width="1.5" />
          </svg>
          <span class="t-stat">{g.n}×</span>
          <span class="t-stat">avg {g.avg}ms</span>
          <span class="t-stat">p95 {g.p95}ms</span>
        </div>
      </div>
    {/each}
    {#if gruppi.length === 0}<div class="vuoto">Nessun dato.</div>{/if}
  </div>
{:else}

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
{/if}

<style>
  .sto-head { padding: 14px 14px 8px; font-size: 11px; font-weight: 600; letter-spacing: .8px; color: var(--txt-faint); display: flex; align-items: center; gap: 6px; }
  .sto-head .act { margin-left: auto; cursor: pointer; color: var(--txt-dim); padding: 0 3px; border-radius: 4px; }
  .sto-head .act:last-child { margin-left: 4px; }
  .sto-head .act:hover { background: var(--panel-3); color: var(--txt); }
  .seg { font-size: 10.5px; font-weight: 600; cursor: pointer; color: var(--txt-faint); padding: 2px 6px; border-radius: 5px; }
  .seg.on { color: var(--txt); background: var(--panel-3); }
  .seg:first-of-type { margin-left: 6px; }
  .trend { padding: 8px; border-bottom: 1px solid var(--border); }
  .t-url { font-family: var(--mono); font-size: 11.5px; color: var(--txt-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; margin-bottom: 4px; }
  .t-riga { display: flex; align-items: center; gap: 8px; }
  .spark { width: 90px; height: 22px; flex: none; }
  .t-stat { font-size: 11px; color: var(--txt-faint); font-family: var(--mono); }
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
