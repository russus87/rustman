<script>
  // Home dashboard: panoramica del workspace (KPI, attività recente, metodi).
  import { t } from "../lib/i18n.svelte.js";

  let { albero = [], environments = [], storia = [], catene = [],
        onNuovaRichiesta, onApri, onVaiA } = $props();

  // Appiattisce tutte le richieste dell'albero.
  function raccogli(figli, acc) {
    for (const n of figli) {
      if (n.tipo === "cartella") raccogli(n.figli, acc);
      else acc.push(n);
    }
    return acc;
  }
  const richieste = $derived(albero.flatMap((c) => raccogli(c.figli, [])));

  const kpi = $derived([
    { lbl: "Collezioni", val: albero.length, hero: true, vista: "collezioni" },
    { lbl: "Richieste", val: richieste.length, vista: "collezioni" },
    { lbl: "Ambienti", val: environments.length, vista: "ambienti" },
    { lbl: "Flussi", val: catene.length, vista: "run" },
  ]);

  // Distribuzione per metodo HTTP.
  const metodi = $derived.by(() => {
    const m = new Map();
    for (const r of richieste) {
      const k = (r.richiesta?.metodo || "GET").toUpperCase();
      m.set(k, (m.get(k) || 0) + 1);
    }
    const ordine = ["GET", "POST", "PUT", "PATCH", "DELETE"];
    return [...m.entries()]
      .sort((a, b) => (ordine.indexOf(a[0]) - ordine.indexOf(b[0])) || b[1] - a[1])
      .map(([nome, n]) => ({ nome, n, pct: richieste.length ? Math.round((n / richieste.length) * 100) : 0 }));
  });

  const recenti = $derived(storia.slice(0, 7));

  function classeMetodo(m) {
    if (m === "GET") return "get"; if (m === "POST") return "post";
    if (m === "PUT" || m === "PATCH") return "put"; if (m === "DELETE") return "del";
    return "";
  }
  function classeStatus(s) {
    if (!s) return "err";
    if (s < 300) return "ok"; if (s < 400) return "warn"; return "err";
  }
  function quando(ts) {
    if (!ts) return "";
    try {
      const d = new Date(ts);
      const diff = (Date.now() - d.getTime()) / 1000;
      if (diff < 60) return "ora";
      if (diff < 3600) return `${Math.floor(diff / 60)} min fa`;
      if (diff < 86400) return `${Math.floor(diff / 3600)} h fa`;
      return d.toLocaleDateString();
    } catch { return ""; }
  }
</script>

<div class="dash">
  <div class="dash-inner">
    <!-- Intestazione -->
    <div class="dhead">
      <div>
        <h1>Dashboard</h1>
        <p>Panoramica del tuo workspace: collezioni, richieste, ambienti e flussi.</p>
      </div>
      <div class="dactions">
        <button class="da primary" onclick={() => onNuovaRichiesta?.()}>+ Nuova richiesta</button>
        <button class="da" onclick={() => onVaiA?.("run")}>🕸 Flussi</button>
      </div>
    </div>

    <!-- KPI -->
    <div class="kpis">
      {#each kpi as k}
        <button class="card kpi" class:hero={k.hero} onclick={() => onVaiA?.(k.vista)}>
          <span class="klbl">{k.lbl}</span>
          <span class="kval">{k.val}</span>
          <span class="kgo">Apri →</span>
        </button>
      {/each}
    </div>

    <div class="grid2">
      <!-- Attività recente -->
      <div class="card">
        <div class="chead">
          <span class="ctitle">Attività recente</span>
          <button class="clink" onclick={() => onVaiA?.("storia")}>Cronologia</button>
        </div>
        {#if recenti.length}
          <div class="rlist">
            {#each recenti as v}
              <button class="ritem" onclick={() => onApri?.(v)}>
                <span class="m {classeMetodo(v.richiesta?.metodo)}">{v.richiesta?.metodo}</span>
                <span class="rname">{v.richiesta?.nome || v.richiesta?.url}</span>
                <span class="rstat {classeStatus(v.status)}">{v.status || "ERR"}</span>
                <span class="rtime">{quando(v.quando)}</span>
              </button>
            {/each}
          </div>
        {:else}
          <div class="vuoto">Nessuna richiesta inviata ancora.</div>
        {/if}
      </div>

      <!-- Distribuzione metodi -->
      <div class="card">
        <div class="chead"><span class="ctitle">Metodi</span></div>
        {#if metodi.length}
          <div class="mlist">
            {#each metodi as mm}
              <div class="mrow">
                <span class="m {classeMetodo(mm.nome)}">{mm.nome}</span>
                <div class="mbar"><div class="mfill {classeMetodo(mm.nome)}" style="width:{mm.pct}%"></div></div>
                <span class="mn">{mm.n}</span>
              </div>
            {/each}
          </div>
        {:else}
          <div class="vuoto">Nessuna richiesta nel workspace.</div>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .dash { height: 100%; overflow-y: auto; background: var(--bg); }
  .dash-inner { max-width: 1080px; margin: 0 auto; padding: 28px 32px 40px; }
  .dhead { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; margin-bottom: 22px; }
  .dhead h1 { font-size: 26px; font-weight: 700; letter-spacing: -.4px; }
  .dhead p { color: var(--txt-dim); font-size: 13px; margin-top: 4px; }
  .dactions { display: flex; gap: 8px; flex: none; }
  .da { border: 1px solid var(--border-2); background: var(--panel-2); color: var(--txt); border-radius: var(--radius); padding: 9px 14px; font-size: 12.5px; font-weight: 500; cursor: pointer; }
  .da:hover { background: var(--panel-3); }
  .da.primary { background: linear-gradient(145deg,var(--accent-2),var(--accent)); color: var(--accent-contrast); border: none; box-shadow: 0 4px 14px var(--accent-soft); }

  .card { background: var(--panel); border: 1px solid var(--border); border-radius: var(--radius-lg); box-shadow: var(--shadow-soft); padding: 16px 18px; }

  .kpis { display: grid; grid-template-columns: repeat(4, 1fr); gap: 14px; margin-bottom: 16px; }
  .kpi { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; cursor: pointer; text-align: left; transition: transform .12s, border-color .12s; }
  .kpi:hover { transform: translateY(-2px); border-color: var(--accent-line); }
  .kpi .klbl { color: var(--txt-dim); font-size: 12px; }
  .kpi .kval { font-size: 30px; font-weight: 700; letter-spacing: -.5px; margin-top: 4px; }
  .kpi .kgo { color: var(--txt-faint); font-size: 11px; margin-top: 6px; }
  .kpi.hero { background: linear-gradient(150deg, var(--accent), var(--accent-strong)); border-color: transparent; color: var(--accent-contrast); }
  .kpi.hero .klbl, .kpi.hero .kgo { color: color-mix(in srgb, var(--accent-contrast) 82%, transparent); }

  .grid2 { display: grid; grid-template-columns: 1.2fr 1fr; gap: 14px; }
  .chead { display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; }
  .ctitle { font-weight: 600; font-size: 14px; }
  .clink { background: none; border: none; color: var(--accent-2); font-size: 12px; cursor: pointer; }
  .vuoto { color: var(--txt-faint); font-size: 12.5px; padding: 10px 2px; }

  .rlist { display: flex; flex-direction: column; gap: 2px; }
  .ritem { display: flex; align-items: center; gap: 10px; padding: 8px 8px; border-radius: 8px; cursor: pointer; background: none; border: none; text-align: left; width: 100%; color: var(--txt); }
  .ritem:hover { background: var(--panel-3); }
  .m { font-size: 10.5px; font-weight: 700; width: 42px; flex: none; }
  .m.get { color: var(--green); } .m.post { color: #d9a441; } .m.put { color: var(--blue); } .m.del { color: var(--red); }
  .rname { flex: 1; font-size: 12.5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .rstat { font-family: var(--mono); font-size: 11px; font-weight: 700; padding: 1px 7px; border-radius: 6px; }
  .rstat.ok { background: rgba(63,185,80,.15); color: #56d364; }
  .rstat.warn { background: rgba(227,160,8,.15); color: var(--orange); }
  .rstat.err { background: rgba(248,81,73,.15); color: #f8918c; }
  .rtime { color: var(--txt-faint); font-size: 11px; width: 58px; text-align: right; flex: none; }

  .mlist { display: flex; flex-direction: column; gap: 12px; }
  .mrow { display: flex; align-items: center; gap: 10px; }
  .mbar { flex: 1; height: 8px; background: var(--panel-3); border-radius: 6px; overflow: hidden; }
  .mfill { height: 100%; border-radius: 6px; background: var(--accent); }
  .mfill.get { background: var(--green); } .mfill.post { background: #d9a441; } .mfill.put { background: var(--blue); } .mfill.del { background: var(--red); }
  .mn { width: 26px; text-align: right; font-size: 12px; color: var(--txt-dim); font-family: var(--mono); }

  @media (max-width: 860px) {
    .kpis { grid-template-columns: repeat(2, 1fr); }
    .grid2 { grid-template-columns: 1fr; }
  }
</style>
