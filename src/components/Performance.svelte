<script>
  // Pannello Performance funzionante: lancia N richieste con un dato grado di
  // concorrenza sulla richiesta corrente e mostra KPI e grafici reali.
  import * as api from "../lib/api.js";
  import { fmtMs } from "../lib/fasi.js";

  let { richiesta, variabili = null } = $props();

  let n = $state(50); // numero di richieste
  let concorrenza = $state(10); // richieste in volo contemporaneamente
  let modo = $state("count"); // count | durata
  let durataS = $state(10); // durata del test (modo durata)
  let rps = $state(0); // RPS target (0 = massimo)
  let warmupS = $state(0); // warmup scartato
  let profilo = $state("costante"); // costante | spike | soak
  let spikeRps = $state(0); // RPS durante il picco (profilo spike)
  let inCorso = $state(false);
  let errore = $state(null);
  let ris = $state(null); // RisultatoPerf
  let grafico = $state("Latenza"); // Latenza | Istogramma | Fasi
  let avanz = $state(null); // ProgressoPerf mentre il test è in corso
  let ultimeOpzioni = $state(null); // opzioni dell'ultimo test (per il report)
  let esportando = $state(false);
  let esitoExp = $state(null);

  // L'invio del test è una singola chiamata che ritorna solo alla fine: per
  // sapere a che punto siamo si interroga il backend, che tiene i contatori
  // aggiornati mano a mano che le richieste si completano.
  const INTERVALLO_MS = 250;
  async function seguiAvanzamento() {
    while (inCorso) {
      try {
        const p = await api.perfProgresso();
        if (inCorso) avanz = p;
      } catch { /* una lettura persa non è un problema: si riprova */ }
      await new Promise((r) => setTimeout(r, INTERVALLO_MS));
    }
  }

  async function esegui() {
    inCorso = true;
    errore = null;
    avanz = null;
    esitoExp = null;
    const opzioni = {
      concorrenza: Number(concorrenza),
      n: Number(n),
      durata_s: modo === "durata" ? Number(durataS) : 0,
      rps: Number(rps),
      warmup_s: Number(warmupS),
      profilo: modo === "durata" ? profilo : "costante",
      spike_rps: Number(spikeRps),
    };
    seguiAvanzamento();
    try {
      ris = await api.eseguiPerfCfg($state.snapshot(richiesta), opzioni, variabili);
      ultimeOpzioni = { ...opzioni, modo, quando: new Date() };
    } catch (e) {
      errore = String(e);
      ris = null;
    } finally {
      inCorso = false;
      avanz = null;
    }
  }

  // Percentuale di avanzamento: sulle richieste nel modo "count", sul tempo
  // nel modo "durata" (dove il totale non si conosce in anticipo).
  const percentuale = $derived.by(() => {
    if (!avanz) return 0;
    if (avanz.previste > 0) return Math.min(100, (avanz.completate / avanz.previste) * 100);
    if (avanz.totale_ms > 0) return Math.min(100, (avanz.trascorso_ms / avanz.totale_ms) * 100);
    return 0;
  });

  // ---- Esportazioni ----
  function scarica(nome, dati, tipo) {
    const blob = dati instanceof Blob ? dati : new Blob([dati], { type: tipo });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = nome;
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  }
  function stampaData(d) {
    return d ? d.toLocaleString("it-IT") : "";
  }
  // Righe "chiave: valore" della configurazione, condivise da CSV e PDF.
  function parametri() {
    const o = ultimeOpzioni;
    if (!o) return [];
    const p = [["Modo", o.modo === "durata" ? "Durata" : "N richieste"]];
    if (o.modo === "durata") {
      p.push(["Profilo", o.profilo], ["Durata (s)", String(o.durata_s)],
        ["RPS target", o.rps ? String(o.rps) : "massimo"]);
      if (o.profilo === "spike") p.push(["RPS picco", String(o.spike_rps)]);
      p.push(["Warmup (s)", String(o.warmup_s)]);
    } else {
      p.push(["Richieste", String(o.n)]);
    }
    p.push(["Concorrenza", String(o.concorrenza)], ["Eseguito il", stampaData(o.quando)]);
    return p;
  }
  function nomeFile(est) {
    const base = (richiesta?.nome || "richiesta").replace(/[^\w.-]+/g, "-").toLowerCase();
    const q = ultimeOpzioni?.quando ?? new Date();
    const stamp = `${q.getFullYear()}${String(q.getMonth() + 1).padStart(2, "0")}${String(q.getDate()).padStart(2, "0")}-${String(q.getHours()).padStart(2, "0")}${String(q.getMinutes()).padStart(2, "0")}`;
    return `perf-${base}-${stamp}.${est}`;
  }

  // CSV in due blocchi: prima il riepilogo, poi una riga per richiesta.
  // È il formato che Excel e LibreOffice aprono senza chiedere nulla.
  function esportaCsv() {
    if (!ris) return;
    const esc = (v) => `"${String(v ?? "").replace(/"/g, '""')}"`;
    const righe = [["metrica", "valore"].map(esc).join(",")];
    const riepilogo = [
      ["richiesta", richiesta?.nome || ""],
      ["metodo", richiesta?.metodo || ""],
      ["url", richiesta?.url || ""],
      ["richieste_totali", ris.totali],
      ["ok", ris.ok],
      ["errori", ris.errori],
      ["durata_totale_ms", ris.durata_totale_ms],
      ["req_al_secondo", ris.req_al_secondo.toFixed(2)],
      ["latenza_min_ms", ris.latenza_min],
      ["latenza_media_ms", ris.latenza_media.toFixed(2)],
      ["latenza_max_ms", ris.latenza_max],
      ["p50_ms", ris.p50], ["p90_ms", ris.p90], ["p95_ms", ris.p95], ["p99_ms", ris.p99],
      ...parametri().map(([k, v]) => [k.toLowerCase().replace(/[^a-z0-9]+/g, "_"), v]),
    ];
    for (const r of riepilogo) righe.push(r.map(esc).join(","));
    if (fasi.length) {
      righe.push("");
      righe.push(["fase", "occorrenze", "ms_medio", "ms_min", "ms_max", "quota_pct"].map(esc).join(","));
      for (const f of fasi) {
        righe.push([f.nome, f.occorrenze, f.ms_medio.toFixed(2), f.ms_min.toFixed(2),
          f.ms_max.toFixed(2), f.quota.toFixed(2)].map(esc).join(","));
      }
    }
    righe.push("");
    righe.push(["indice", "latenza_ms"].map(esc).join(","));
    ris.latenze.forEach((l, i) => righe.push([i, l].map(esc).join(",")));
    scarica(nomeFile("csv"), righe.join("\n"), "text/csv;charset=utf-8");
    esitoExp = { ok: true, testo: "CSV esportato" };
  }

  // Il PDF (grafici inclusi) è generato dal backend e arriva in base64.
  async function esportaPdf() {
    if (!ris || esportando) return;
    esportando = true;
    esitoExp = null;
    try {
      const b64 = await api.esportaPerfPdf(
        $state.snapshot(ris),
        richiesta?.nome || "Richiesta",
        `${richiesta?.metodo || ""} ${richiesta?.url || ""}`.trim(),
        parametri(),
      );
      const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
      scarica(nomeFile("pdf"), new Blob([bytes], { type: "application/pdf" }));
      esitoExp = { ok: true, testo: "PDF esportato" };
    } catch (e) {
      esitoExp = { ok: false, testo: `PDF non riuscito: ${e}` };
    } finally {
      esportando = false;
    }
  }

  // Dimensioni del grafico (coordinate SVG).
  const W = 620, H = 200;
  const pad = { l: 44, r: 10, t: 12, b: 22 };

  // Punti della spezzata per il grafico delle latenze.
  function lineaPunti(lat) {
    if (!lat.length) return "";
    const max = Math.max(...lat, 1);
    const iw = W - pad.l - pad.r;
    const ih = H - pad.t - pad.b;
    const num = lat.length;
    return lat
      .map((v, i) => {
        const x = pad.l + (num === 1 ? 0 : (i / (num - 1)) * iw);
        const y = pad.t + ih - (v / max) * ih;
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
  }

  // Conteggi per fascia di latenza (istogramma).
  function istogramma(lat, nb = 12) {
    if (!lat.length) return { buckets: [], min: 0, passo: 1 };
    const min = Math.min(...lat), max = Math.max(...lat);
    const range = Math.max(max - min, 1);
    const buckets = new Array(nb).fill(0);
    for (const v of lat) {
      let idx = Math.floor(((v - min) / range) * nb);
      if (idx >= nb) idx = nb - 1;
      buckets[idx]++;
    }
    return { buckets, min, passo: range / nb };
  }

  // ---- Fasi dichiarate dal servizio ----
  // Se le risposte raccontano quanto è durata ogni tappa del lavoro, il
  // backend le aggrega su tutto il test: qui si vede quanto pesa ciascuna sul
  // tempo di una richiesta. Se non ne parlano, il tab non compare.
  const fasi = $derived(ris?.fasi ?? []);
  const fasiSomma = $derived(fasi.reduce((s, f) => s + f.ms_medio, 0));
  const fasiMax = $derived(Math.max(...fasi.map((f) => f.ms_medio), 0.001));
  const grafici = $derived(fasi.length ? ["Latenza", "Istogramma", "Fasi"] : ["Latenza", "Istogramma"]);
  $effect(() => {
    if (grafico === "Fasi" && fasi.length === 0) grafico = "Latenza";
  });

  const maxLat = $derived(ris ? Math.max(...ris.latenze, 1) : 1);
  const histo = $derived(ris ? istogramma(ris.latenze) : { buckets: [] });
  const maxBucket = $derived(histo.buckets.length ? Math.max(...histo.buckets, 1) : 1);
</script>

<div class="perf">
  <div class="perf-tabs">
    <div class="ptab active">Performance</div>
  </div>
  <div class="perf-body">
    <!-- Form di configurazione -->
    <div class="perf-form">
      <label>Modo
        <select bind:value={modo}>
          <option value="count">N richieste</option>
          <option value="durata">Durata</option>
        </select>
      </label>
      {#if modo === "count"}
        <label>Richieste<input type="number" min="1" max="50000" bind:value={n} /></label>
      {:else}
        <label>Profilo
          <select bind:value={profilo}>
            <option value="costante">Costante</option>
            <option value="spike">Spike</option>
            <option value="soak">Soak</option>
          </select>
        </label>
        <label>Durata (s)<input type="number" min="1" max="86400" bind:value={durataS} /></label>
        <label>RPS target<input type="number" min="0" max="100000" bind:value={rps} /></label>
        {#if profilo === "spike"}
          <label>RPS picco<input type="number" min="0" max="100000" bind:value={spikeRps} /></label>
        {/if}
        <label>Warmup (s)<input type="number" min="0" max="600" bind:value={warmupS} /></label>
      {/if}
      <label>Concorrenza<input type="number" min="1" max="256" bind:value={concorrenza} /></label>
      <button class="btn btn-send" onclick={esegui} disabled={inCorso}>
        {inCorso ? "In corso…" : "Esegui"}
      </button>
    </div>

    {#if inCorso && avanz}
      <div class="avanz">
        <div class="av-barra">
          <!-- Nel modo "durata" la barra segue il tempo, non le richieste. -->
          <div class="av-riemp" style="width:{percentuale.toFixed(1)}%"></div>
        </div>
        <div class="av-righe">
          <span class="av-perc">{percentuale.toFixed(0)}%</span>
          <span>
            {avanz.completate.toLocaleString("it-IT")}
            {#if avanz.previste > 0}/ {avanz.previste.toLocaleString("it-IT")}{/if}
            richieste
          </span>
          <span class="av-ok">{avanz.ok.toLocaleString("it-IT")} ok</span>
          {#if avanz.errori > 0}<span class="av-ko">{avanz.errori.toLocaleString("it-IT")} errori</span>{/if}
          <span>{avanz.req_al_secondo.toFixed(1)} req/s</span>
          <span>media {avanz.latenza_media.toFixed(0)} ms</span>
          <span>ultima {avanz.latenza_ultima} ms</span>
          <span class="av-t">
            {(avanz.trascorso_ms / 1000).toFixed(1)}s
            {#if avanz.totale_ms > 0}/ {(avanz.totale_ms / 1000).toFixed(0)}s{/if}
          </span>
        </div>
      </div>
    {/if}

    {#if errore}
      <div class="err-box">{errore}</div>
    {:else if !ris}
      <div class="placeholder" style="height:auto;padding:24px 0">
        <div class="big">Nessun test eseguito</div>
        <div>Imposta i parametri e premi <b>Esegui</b> sulla richiesta corrente.</div>
      </div>
    {:else}
      <div class="exp-bar">
        {#if esitoExp}
          <span class="exp-esito" class:ko={!esitoExp.ok}>{esitoExp.testo}</span>
        {/if}
        <span class="exp-sp"></span>
        <button class="exp-btn" onclick={esportaCsv} title="Riepilogo e latenze in CSV">Esporta CSV</button>
        <button class="exp-btn" onclick={esportaPdf} disabled={esportando}
          title="Report PDF con KPI, percentili e grafici">
          {esportando ? "Genero…" : "Esporta PDF"}
        </button>
      </div>

      <!-- KPI principali -->
      <div class="kpi-grid">
        <div class="kpi"><div class="lbl">Richieste</div><div class="val">{ris.totali}</div></div>
        <div class="kpi"><div class="lbl">OK / Errori</div><div class="val">{ris.ok} / {ris.errori}</div></div>
        <div class="kpi"><div class="lbl">Req/s</div><div class="val">{ris.req_al_secondo.toFixed(1)}</div></div>
        <div class="kpi"><div class="lbl">Durata</div><div class="val">{(ris.durata_totale_ms / 1000).toFixed(2)}s</div></div>
      </div>

      <!-- Metriche di latenza -->
      <div class="metric-grid">
        <div class="metric"><div class="ml">Min</div><div class="mv">{ris.latenza_min}<small>ms</small></div></div>
        <div class="metric"><div class="ml">Media</div><div class="mv">{ris.latenza_media.toFixed(0)}<small>ms</small></div></div>
        <div class="metric"><div class="ml">P95</div><div class="mv">{ris.p95}<small>ms</small></div></div>
        <div class="metric"><div class="ml">Max</div><div class="mv">{ris.latenza_max}<small>ms</small></div></div>
      </div>

      <!-- Percentili extra -->
      <div class="perc-row">
        <span>P50 <b>{ris.p50} ms</b></span>
        <span>P90 <b>{ris.p90} ms</b></span>
        <span>P95 <b>{ris.p95} ms</b></span>
        <span>P99 <b>{ris.p99} ms</b></span>
      </div>

      <div class="chart-tabs">
        {#each grafici as c}
          <div class="ctab" class:active={grafico === c} onclick={() => (grafico = c)}>{c}</div>
        {/each}
      </div>

      <div class="big-chart">
        {#if grafico === "Latenza"}
          <svg viewBox="0 0 {W} {H}" width="100%" preserveAspectRatio="none" style="display:block">
            <defs>
              <linearGradient id="areaPerf" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stop-color="var(--accent-2)" stop-opacity="0.30" />
                <stop offset="100%" stop-color="var(--accent-2)" stop-opacity="0" />
              </linearGradient>
            </defs>
            <!-- assi Y -->
            <g class="axis" text-anchor="end">
              <text x={pad.l - 6} y={pad.t + 4}>{maxLat}</text>
              <text x={pad.l - 6} y={H - pad.b}>0</text>
            </g>
            <line x1={pad.l} y1={pad.t} x2={pad.l} y2={H - pad.b} stroke="#1e1e2a" />
            <line x1={pad.l} y1={H - pad.b} x2={W - pad.r} y2={H - pad.b} stroke="#1e1e2a" />
            <polyline points={lineaPunti(ris.latenze)} fill="none" stroke="#9b80ff" stroke-width="1.6" />
          </svg>
          <div class="chart-cap">latenza (ms) per richiesta, in ordine di completamento</div>
        {:else if grafico === "Istogramma"}
          <svg viewBox="0 0 {W} {H}" width="100%" preserveAspectRatio="none" style="display:block">
            <line x1={pad.l} y1={pad.t} x2={pad.l} y2={H - pad.b} stroke="#1e1e2a" />
            <line x1={pad.l} y1={H - pad.b} x2={W - pad.r} y2={H - pad.b} stroke="#1e1e2a" />
            {#each histo.buckets as conteggio, i}
              {@const iw = W - pad.l - pad.r}
              {@const ih = H - pad.t - pad.b}
              {@const bw = iw / histo.buckets.length}
              {@const bh = (conteggio / maxBucket) * ih}
              <rect
                x={(pad.l + i * bw + 1).toFixed(1)}
                y={(pad.t + ih - bh).toFixed(1)}
                width={(bw - 2).toFixed(1)}
                height={bh.toFixed(1)}
                fill="var(--accent-2)"
                rx="1"
              />
            {/each}
          </svg>
          <div class="chart-cap">distribuzione delle latenze (numero di richieste per fascia)</div>
        {:else}
          <div class="fasi-testa">
            <span>somma delle fasi <b>{fmtMs(fasiSomma)}</b></span>
            <span>latenza media <b>{fmtMs(ris.latenza_media)}</b></span>
            {#if ris.latenza_media > fasiSomma}
              <span title="Latenza che nessuna fase spiega: rete, attesa in coda, serializzazione.">
                altro <b>{fmtMs(ris.latenza_media - fasiSomma)}</b>
              </span>
            {/if}
          </div>
          <div class="fasi-elenco">
            {#each fasi as f}
              <div class="fp-voce">
                <div class="fp-riga">
                  <span class="fp-nome" title={f.nome}>{f.nome}</span>
                  <span class="fp-ms">{fmtMs(f.ms_medio)}</span>
                  <span class="fp-quota" class:top={f.ms_medio === fasiMax}>{f.quota.toFixed(1)}%</span>
                </div>
                <div class="fp-barra">
                  <span class="fp-b" class:top={f.ms_medio === fasiMax} style="width:{(f.ms_medio / fasiMax) * 100}%"></span>
                </div>
                <div class="fp-det">{f.occorrenze} risposte · min {fmtMs(f.ms_min)} · max {fmtMs(f.ms_max)}</div>
              </div>
            {/each}
          </div>
          <div class="chart-cap">
            media per richiesta delle fasi dichiarate nel corpo della risposta; la
            percentuale è sulla somma delle medie
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  /* Avanzamento del test in corso */
  .avanz { margin-bottom: 16px; }
  .av-barra {
    height: 6px; border-radius: 3px; background: var(--panel-3); overflow: hidden;
  }
  .av-riemp {
    height: 100%; border-radius: 3px;
    background: linear-gradient(90deg, var(--accent), var(--accent-2));
    transition: width .2s linear;
  }
  .av-righe {
    display: flex; flex-wrap: wrap; align-items: center; gap: 4px 14px; margin-top: 8px;
    font-family: var(--mono); font-size: 11.5px; color: var(--txt-dim);
  }
  .av-perc { color: var(--txt); font-weight: 600; }
  .av-ok { color: var(--green); }
  .av-ko { color: var(--red); }
  .av-t { margin-left: auto; color: var(--txt-faint); }
  /* Barra delle esportazioni */
  .exp-bar { display: flex; align-items: center; gap: 10px; margin-bottom: 12px; }
  .exp-sp { flex: 1; }
  .exp-esito { font-size: 11.5px; color: var(--green); }
  .exp-esito.ko { color: var(--red); }
  .exp-btn {
    background: var(--panel-2); color: var(--txt); border: 1px solid var(--border-2);
    border-radius: 7px; padding: 6px 12px; font-size: 12px; cursor: pointer;
  }
  .exp-btn:hover:not(:disabled) { background: var(--panel-3); border-color: var(--accent-line); }
  .exp-btn:disabled { opacity: .55; cursor: default; }
  .perf-form {
    display: flex;
    align-items: flex-end;
    gap: 12px;
    margin-bottom: 14px;
  }
  .perf-form label {
    display: flex;
    flex-direction: column;
    gap: 5px;
    color: var(--txt-dim);
    font-size: 12px;
  }
  .perf-form input,
  .perf-form select {
    width: 90px;
    background: var(--panel-2);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 8px 10px;
    color: var(--txt);
    font-family: var(--mono);
    outline: none;
  }
  .perf-form input:focus,
  .perf-form select:focus {
    border-color: var(--accent);
  }
  .perc-row {
    display: flex;
    gap: 18px;
    margin-bottom: 14px;
    color: var(--txt-dim);
    font-size: 12.5px;
  }
  .perc-row b {
    color: var(--txt);
  }
  /* Fasi dichiarate dal servizio */
  .fasi-testa {
    display: flex; flex-wrap: wrap; gap: 4px 18px; margin-bottom: 10px;
    color: var(--txt-dim); font-size: 12px;
  }
  .fasi-testa b { color: var(--txt); font-family: var(--mono); }
  /* Ogni fase è un blocco su tre righe (nome+numeri, barra, dettaglio): il
     pannello è stretto e una tabella a colonne fisse ci starebbe male. */
  .fasi-elenco { display: flex; flex-direction: column; gap: 12px; }
  .fp-riga { display: flex; align-items: baseline; gap: 10px; }
  .fp-nome {
    flex: 1; min-width: 0; color: var(--txt); font-family: var(--mono); font-size: 12px;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .fp-barra { margin-top: 4px; }
  .fp-b {
    display: block; height: 10px; border-radius: 3px; min-width: 2px;
    background: linear-gradient(90deg, var(--accent-2), var(--accent)); opacity: .55;
  }
  .fp-b.top { opacity: 1; }
  .fp-ms, .fp-quota {
    font-family: var(--mono); font-size: 12px; color: var(--txt-dim); white-space: nowrap;
  }
  .fp-quota { min-width: 46px; text-align: right; color: var(--txt-faint); }
  .fp-quota.top { color: var(--txt); font-weight: 600; }
  .fp-det {
    margin-top: 4px; color: var(--txt-faint); font-family: var(--mono); font-size: 10.5px;
  }
  .chart-cap {
    color: var(--txt-faint);
    font-size: 11px;
    margin-top: 6px;
    text-align: center;
  }
</style>
