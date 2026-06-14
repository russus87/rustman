<script>
  // Pannello Performance funzionante: lancia N richieste con un dato grado di
  // concorrenza sulla richiesta corrente e mostra KPI e grafici reali.
  import * as api from "../lib/api.js";

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
  let grafico = $state("Latenza"); // Latenza | Istogramma

  async function esegui() {
    inCorso = true;
    errore = null;
    try {
      const opzioni = {
        concorrenza: Number(concorrenza),
        n: Number(n),
        durata_s: modo === "durata" ? Number(durataS) : 0,
        rps: Number(rps),
        warmup_s: Number(warmupS),
        profilo: modo === "durata" ? profilo : "costante",
        spike_rps: Number(spikeRps),
      };
      ris = await api.eseguiPerfCfg($state.snapshot(richiesta), opzioni, variabili);
    } catch (e) {
      errore = String(e);
      ris = null;
    } finally {
      inCorso = false;
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

    {#if errore}
      <div class="err-box">{errore}</div>
    {:else if !ris}
      <div class="placeholder" style="height:auto;padding:24px 0">
        <div class="big">Nessun test eseguito</div>
        <div>Imposta i parametri e premi <b>Esegui</b> sulla richiesta corrente.</div>
      </div>
    {:else}
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
        {#each ["Latenza", "Istogramma"] as c}
          <div class="ctab" class:active={grafico === c} onclick={() => (grafico = c)}>{c}</div>
        {/each}
      </div>

      <div class="big-chart">
        {#if grafico === "Latenza"}
          <svg viewBox="0 0 {W} {H}" width="100%" preserveAspectRatio="none" style="display:block">
            <defs>
              <linearGradient id="areaPerf" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stop-color="#8b6dff" stop-opacity="0.30" />
                <stop offset="100%" stop-color="#8b6dff" stop-opacity="0" />
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
        {:else}
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
                fill="#8b6dff"
                rx="1"
              />
            {/each}
          </svg>
          <div class="chart-cap">distribuzione delle latenze (numero di richieste per fascia)</div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
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
  .chart-cap {
    color: var(--txt-faint);
    font-size: 11px;
    margin-top: 6px;
    text-align: center;
  }
</style>
