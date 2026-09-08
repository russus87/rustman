<script>
  // Mostra l'esito di una catena di run (un passo per riga, con test e log).
  // Sui nodi eseguiti in loop aggiunge statistiche, andamento e distribuzione
  // delle latenze, più una fascia di confronto quando i nodi con loop sono due
  // o più: è il caso tipico del benchmark A/B fra due implementazioni.
  import * as api from "../lib/api.js";

  let { titolo, risultati = [] } = $props();

  const passati = $derived(risultati.filter((r) => r.ok).length);

  // Nodi con dei tempi da confrontare.
  const conCiclo = $derived(risultati.filter((r) => r.ciclo?.tempi?.length));
  const confronto = $derived.by(() => {
    if (conCiclo.length < 2) return null;
    const base = conCiclo[0].ciclo.p50 || 0;
    const peggiore = Math.max(...conCiclo.map((r) => r.ciclo.p50 || 0), 1);
    return {
      base,
      righe: conCiclo.map((r, i) => ({
        nome: r.nome,
        ciclo: r.ciclo,
        primo: i === 0,
        delta: base > 0 ? ((r.ciclo.p50 - base) / base) * 100 : 0,
        larghezza: ((r.ciclo.p50 || 0) / peggiore) * 100,
      })),
    };
  });

  // ---- Grafici (stesse coordinate del pannello Performance) ----
  const W = 620, H = 90;
  const pad = { l: 40, r: 8, t: 8, b: 14 };

  // L'asse verticale va da min a max, non da zero: con latenze tutte attorno
  // allo stesso valore una scala 0-max le schiaccerebbe in una riga piatta.
  function spezzata(tempi) {
    if (!tempi.length) return "";
    const min = Math.min(...tempi), max = Math.max(...tempi);
    const span = Math.max(max - min, 1);
    const iw = W - pad.l - pad.r, ih = H - pad.t - pad.b;
    return tempi
      .map((v, i) => {
        const x = pad.l + (tempi.length === 1 ? 0 : (i / (tempi.length - 1)) * iw);
        const y = pad.t + ih - ((v - min) / span) * ih;
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
  }
  function istogramma(tempi, nb = 16) {
    if (!tempi.length) return { fasce: [], min: 0, max: 0 };
    const min = Math.min(...tempi), max = Math.max(...tempi);
    const span = Math.max(max - min, 1);
    const fasce = new Array(nb).fill(0);
    for (const v of tempi) fasce[Math.min(nb - 1, Math.floor(((v - min) / span) * nb))]++;
    return { fasce, min, max };
  }
  function ms(v) {
    return v == null ? "—" : `${Math.round(v)} ms`;
  }

  function scarica(nome, contenuto, tipo) {
    const blob = contenuto instanceof Blob ? contenuto : new Blob([contenuto], { type: tipo });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url; a.download = nome;
    document.body.appendChild(a); a.click(); a.remove(); URL.revokeObjectURL(url);
  }
  function esportaJson() {
    scarica("run.json", JSON.stringify(risultati, null, 2), "application/json");
  }

  // ---- Report PDF (confronto + scheda per nodo), generato dal backend ----
  let pdfInCorso = $state(false);
  let esitoPdf = $state(null);
  function nomeFile(est) {
    const base = (titolo || "run").replace(/[^\w.-]+/g, "-").toLowerCase();
    const d = new Date();
    const p2 = (n) => String(n).padStart(2, "0");
    return `${base}-${d.getFullYear()}${p2(d.getMonth() + 1)}${p2(d.getDate())}-${p2(d.getHours())}${p2(d.getMinutes())}.${est}`;
  }
  async function esportaPdf() {
    if (pdfInCorso) return;
    pdfInCorso = true;
    esitoPdf = null;
    try {
      const sezioni = risultati
        .filter((r) => r.ciclo)
        .map((r) => ({
          nome: r.nome ?? "",
          ok: !!r.ok,
          giri: r.ciclo.giri ?? 0,
          ok_giri: r.ciclo.ok ?? 0,
          falliti: r.ciclo.falliti ?? 0,
          concorrenza: r.ciclo.concorrenza ?? 1,
          sorgente: r.ciclo.sorgente ?? "",
          uscita: r.ciclo.uscita ?? "",
          tempi: r.ciclo.tempi ?? [],
          test_ok: r.ciclo.testOk ?? 0,
          test_tot: r.ciclo.testTot ?? 0,
          problemi: (r.ciclo.problemi ?? []).map(
            (p) => `giro ${p.giro}: ${p.errore ?? `status ${p.status ?? "?"}`}${p.test?.length ? ` · ${p.test.join(" · ")}` : ""}`,
          ),
        }));
      if (!sezioni.length) {
        esitoPdf = { ok: false, testo: "nessun nodo con loop da riportare" };
        return;
      }
      const b64 = await api.esportaRunPdf(
        titolo || "Flusso",
        `${sezioni.length} nod${sezioni.length === 1 ? "o" : "i"} con loop · ${new Date().toLocaleString("it-IT")}`,
        sezioni,
      );
      const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
      scarica(nomeFile("pdf"), new Blob([bytes], { type: "application/pdf" }));
      esitoPdf = { ok: true, testo: "PDF esportato" };
    } catch (e) {
      esitoPdf = { ok: false, testo: `PDF non riuscito: ${e}` };
    } finally {
      pdfInCorso = false;
    }
  }
  function esportaCsv() {
    const esc = (s) => `"${String(s ?? "").replace(/"/g, '""')}"`;
    const righe = [["passo", "esito", "status", "tempo_ms", "test_ok", "test_tot", "giri", "giri_ok", "errore"].map(esc)];
    for (const r of risultati) {
      // Con un ciclo i test sono contati sui giri, non sul singolo invio.
      const tot = r.ciclo ? r.ciclo.testTot : (r.tests || []).length;
      const ok = r.ciclo ? r.ciclo.testOk : (r.tests || []).filter((t) => t.passato).length;
      righe.push([r.nome, r.saltato ? "skip" : r.ok ? "ok" : "fail", r.status ?? "", r.tempo ?? "",
        ok, tot, r.ciclo?.giri ?? "", r.ciclo?.ok ?? "", r.errore ?? ""].map(esc));
    }
    // Secondo blocco: la latenza di ogni singolo giro, per rianalizzarla fuori.
    const conTempi = risultati.filter((r) => r.ciclo?.tempi?.length);
    if (conTempi.length) {
      righe.push([""]);
      righe.push(["passo", "giro", "latenza_ms"].map(esc));
      for (const r of conTempi) {
        r.ciclo.tempi.forEach((t, i) => righe.push([r.nome, i, t].map(esc)));
      }
    }
    scarica("run.csv", righe.map((r) => r.join(",")).join("\n"), "text/csv");
  }
</script>

<div class="rr">
  <div class="rr-head">
    <span class="t">{titolo}</span>
    <span class="riepilogo {passati === risultati.length ? 'ok' : 'ko'}">
      {passati}/{risultati.length} passi ok
    </span>
    {#if esitoPdf}<span class="exp-esito" class:ko={!esitoPdf.ok}>{esitoPdf.testo}</span>{/if}
    {#if risultati.length}
      <span class="exp" onclick={esportaCsv} title="Riepilogo e latenza di ogni giro">CSV</span>
      <span class="exp" onclick={esportaJson} title="Esporta JSON">JSON</span>
      {#if conCiclo.length}
        <span class="exp pdf" class:disab={pdfInCorso} onclick={esportaPdf}
          title="Report con confronto, statistiche e grafici">{pdfInCorso ? "…" : "PDF"}</span>
      {/if}
    {/if}
  </div>

  <div class="rr-body">
    {#if confronto}
      <div class="conf">
        <div class="conf-t">Confronto</div>
        <table class="conf-tab">
          <thead>
            <tr><th>nodo</th><th>giri</th><th>ok</th><th>P50</th><th>P95</th><th>media</th><th>vs 1°</th></tr>
          </thead>
          <tbody>
            {#each confronto.righe as c}
              <tr>
                <td class="cn">{c.nome}</td>
                <td>{c.ciclo.giri}</td>
                <td>{c.ciclo.ok}</td>
                <td>{ms(c.ciclo.p50)}</td>
                <td>{ms(c.ciclo.p95)}</td>
                <td>{ms(c.ciclo.tempoMedio)}</td>
                <td class="cd" class:peggio={c.delta > 0.5} class:meglio={!c.primo && c.delta <= 0.5}>
                  {c.primo ? "riferimento" : `${c.delta > 0 ? "+" : ""}${c.delta.toFixed(1)}%`}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
        <div class="conf-barre">
          {#each confronto.righe as c}
            <div class="cb-riga">
              <span class="cb-nome" title={c.nome}>{c.nome}</span>
              <div class="cb-pista"><div class="cb-riemp" style="width:{c.larghezza.toFixed(1)}%"></div></div>
              <span class="cb-val">{ms(c.ciclo.p50)}</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    {#each risultati as r, i}
      <div class="passo" class:ko={!r.ok} class:salt={r.saltato}>
        <div class="riga1">
          <span class="badge {r.saltato ? 'salt' : r.ok ? 'ok' : 'ko'}">{r.saltato ? "SKIP" : r.ok ? "OK" : "FAIL"}</span>
          <span class="num">#{i + 1}</span>
          <span class="nome">{r.nome}</span>
          <span class="sp"></span>
          {#if r.status}<span class="meta">{r.status}</span>{/if}
          {#if r.tempo != null}<span class="meta">{r.tempo} ms</span>{/if}
        </div>
        {#if r.errore}<div class="errore">{r.errore}</div>{/if}
        {#if r.ciclo}
          {@const cl = r.ciclo}
          <div class="ciclo">
            <span class="cb">↻ {cl.giri}{#if cl.previsti != null && cl.previsti !== cl.giri}/{cl.previsti}{/if} giri</span>
            <span>{cl.sorgente}</span>
            {#if cl.concorrenza > 1}<span>{cl.concorrenza} in parallelo</span>{/if}
            <span class="cok">{cl.ok} ok</span>
            {#if cl.falliti}<span class="cko">{cl.falliti} falliti</span>{/if}
            {#if cl.tempoMedio != null}<span>{cl.tempoMedio} ms medi · min {cl.tempoMin} · max {cl.tempoMax}</span>{/if}
            {#if cl.testTot}<span>test {cl.testOk}/{cl.testTot}</span>{/if}
            <span class="cus">{cl.uscita}</span>
          </div>
          {#if cl.tempi?.length}
            {@const h = istogramma(cl.tempi)}
            <div class="perc-riga">
              min {ms(cl.tempoMin)} · P50 {ms(cl.p50)} · P90 {ms(cl.p90)} · P95 {ms(cl.p95)} · P99 {ms(cl.p99)} · max {ms(cl.tempoMax)}
            </div>
            <!-- Le etichette dell'asse stanno fuori dall'SVG: con
                 preserveAspectRatio="none" il testo verrebbe stirato con il
                 disegno. La scala parte dal minimo, non da zero. -->
            <div class="graf">
              <div class="graf-riga">
                <div class="graf-ax">
                  <span>{cl.tempoMax}</span>
                  <span>{cl.tempoMin}</span>
                </div>
                <svg viewBox="0 0 {W} {H}" width="100%" preserveAspectRatio="none">
                  <line x1={pad.l} y1={pad.t} x2={pad.l} y2={H - pad.b} stroke="var(--border)" stroke-width="0.7" />
                  <line x1={pad.l} y1={H - pad.b} x2={W - pad.r} y2={H - pad.b} stroke="var(--border)" stroke-width="0.7" />
                  <polyline points={spezzata(cl.tempi)} fill="none" stroke="var(--accent-2)" stroke-width="1.2" />
                </svg>
              </div>
              <span class="graf-cap">latenza per giro · {cl.tempoMin}–{cl.tempoMax} ms</span>
            </div>
            <div class="graf">
              <svg viewBox="0 0 {W} {H}" width="100%" preserveAspectRatio="none">
                <line x1={pad.l} y1={H - pad.b} x2={W - pad.r} y2={H - pad.b} stroke="var(--border)" stroke-width="0.7" />
                {#each h.fasce as n, k}
                  {@const iw = W - pad.l - pad.r}
                  {@const ih = H - pad.t - pad.b}
                  {@const bw = iw / h.fasce.length}
                  {@const bh = (n / Math.max(...h.fasce, 1)) * ih}
                  <rect x={(pad.l + k * bw + 1).toFixed(1)} y={(pad.t + ih - bh).toFixed(1)}
                    width={(bw - 2).toFixed(1)} height={bh.toFixed(1)} fill="var(--accent-2)" rx="1" />
                {/each}
              </svg>
              <span class="graf-cap">distribuzione · {h.min}–{h.max} ms</span>
            </div>
          {/if}
          {#each cl.problemi ?? [] as p}
            <div class="test">
              <span class="te ko">giro {p.giro}</span>
              <span>{p.errore ?? `status ${p.status ?? "?"}`}</span>
              {#if p.test?.length}<span class="det">{p.test.join(" · ")}</span>{/if}
            </div>
          {/each}
          {#if cl.falliti > (cl.problemi?.length ?? 0)}
            <div class="test"><span class="det">…e altri {cl.falliti - cl.problemi.length} giri falliti</span></div>
          {/if}
        {/if}
        {#if r.catture && Object.keys(r.catture).length}
          <div class="catture">
            {#each Object.entries(r.catture) as [k, v]}<span class="cap">⇲ {k} = {v}</span>{/each}
          </div>
        {/if}
        {#each r.tests as t}
          <div class="test">
            <span class="te {t.passato ? 'ok' : 'ko'}">{t.passato ? "PASS" : "FAIL"}</span>
            <span>{t.descrizione}</span>
            {#if t.dettaglio}<span class="det">{t.dettaglio}</span>{/if}
          </div>
        {/each}
        {#if r.logs?.length}
          <div class="logs">{r.logs.join("\n")}</div>
        {/if}
      </div>
    {/each}
    {#if risultati.length === 0}
      <div class="placeholder"><div>Nessun risultato.</div></div>
    {/if}
  </div>
</div>

<style>
  .rr { flex: 1; display: flex; flex-direction: column; min-height: 0; }
  .rr-head { display: flex; align-items: center; gap: 12px; padding: 12px 16px; border-bottom: 1px solid var(--border); }
  .rr-head .t { font-family: var(--mono); font-size: 12.5px; }
  .riepilogo { margin-left: auto; font-weight: 700; font-size: 12px; padding: 3px 9px; border-radius: 6px; }
  .riepilogo.ok { color: #56d364; background: rgba(63,185,80,.15); }
  .riepilogo.ko { color: #f8918c; background: rgba(248,81,73,.15); }
  .exp { cursor: pointer; font-size: 11px; color: var(--txt-faint); border: 1px solid var(--border); border-radius: 5px; padding: 2px 8px; font-family: var(--mono); }
  .exp:hover { color: var(--accent); border-color: var(--accent); }
  .rr-body { flex: 1; overflow: auto; padding: 12px 16px; }
  .passo { border: 1px solid var(--border); border-radius: 9px; padding: 10px 12px; margin-bottom: 10px; background: var(--panel-2); }
  .passo.ko { border-color: rgba(248,81,73,.4); }
  .riga1 { display: flex; align-items: center; gap: 10px; }
  .badge { font-family: var(--mono); font-weight: 700; font-size: 11px; padding: 2px 8px; border-radius: 5px; }
  .badge.ok { color: #56d364; background: rgba(63,185,80,.15); }
  .badge.ko { color: #f8918c; background: rgba(248,81,73,.15); }
  .badge.salt { color: var(--txt-faint); background: var(--panel-3); }
  .passo.salt { opacity: .7; }
  /* Fascia di confronto fra i nodi con loop. */
  .conf { padding: 12px 14px; border-bottom: 1px solid var(--border); background: var(--panel); }
  .conf-t { font-weight: 600; font-size: 12.5px; margin-bottom: 8px; }
  .conf-tab { width: 100%; border-collapse: collapse; font-size: 11.5px; font-family: var(--mono); }
  .conf-tab th {
    text-align: left; color: var(--txt-faint); font-weight: 500; font-size: 10.5px;
    padding: 0 8px 4px 0; border-bottom: 1px solid var(--border);
  }
  .conf-tab td { padding: 4px 8px 4px 0; color: var(--txt-dim); white-space: nowrap; }
  .conf-tab td.cn { color: var(--txt); width: 40%; max-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .conf-tab td.cd { color: var(--txt-faint); }
  .conf-tab td.cd.peggio { color: var(--red); }
  .conf-tab td.cd.meglio { color: var(--green); }
  .conf-barre { margin-top: 10px; display: flex; flex-direction: column; gap: 5px; }
  .cb-riga { display: flex; align-items: center; gap: 8px; font-size: 10.5px; }
  .cb-nome { width: 34%; color: var(--txt-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cb-pista { flex: 1; height: 8px; background: var(--panel-3); border-radius: 4px; overflow: hidden; }
  .cb-riemp { height: 100%; background: linear-gradient(90deg, var(--accent), var(--accent-2)); border-radius: 4px; }
  .cb-val { width: 70px; text-align: right; color: var(--txt-faint); font-family: var(--mono); }
  /* Percentili e grafici di un nodo in loop. */
  .perc-riga { margin-top: 5px; font-family: var(--mono); font-size: 11px; color: var(--txt-dim); }
  .graf { margin-top: 6px; }
  .graf svg { display: block; height: 90px; width: 100%; }
  .graf-cap { font-size: 10px; color: var(--txt-faint); }
  .graf-riga { display: flex; align-items: stretch; gap: 6px; }
  .graf-ax {
    display: flex; flex-direction: column; justify-content: space-between;
    padding: 2px 0 14px; font-family: var(--mono); font-size: 9px;
    color: var(--txt-faint); text-align: right; min-width: 42px;
  }
  .graf-riga svg { flex: 1; }
  .exp-esito { font-size: 11px; color: var(--green); margin-right: 8px; }
  .exp-esito.ko { color: var(--red); }
  .exp.pdf.disab { opacity: .55; pointer-events: none; }
  /* Riepilogo di un nodo eseguito in loop. */
  .ciclo {
    display: flex; flex-wrap: wrap; align-items: center; gap: 4px 12px; margin-top: 6px;
    font-size: 11px; color: var(--txt-dim); font-family: var(--mono);
  }
  .ciclo .cb { color: var(--accent-2); }
  .ciclo .cok { color: var(--green); }
  .ciclo .cko { color: var(--red); }
  .ciclo .cus { color: var(--txt-faint); font-style: italic; }
  .catture { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 6px; }
  .cap { font-family: var(--mono); font-size: 11px; color: var(--green); background: rgba(63,185,80,.12); padding: 1px 7px; border-radius: 5px; }
  .num { color: var(--txt-faint); font-family: var(--mono); font-size: 11px; }
  .nome { font-weight: 600; }
  .sp { flex: 1; }
  .meta { color: var(--txt-dim); font-family: var(--mono); font-size: 12px; }
  .errore { color: #f8918c; font-family: var(--mono); font-size: 12px; margin-top: 6px; }
  .test { display: flex; align-items: center; gap: 8px; font-size: 12px; margin-top: 6px; }
  .te { font-family: var(--mono); font-weight: 700; font-size: 10.5px; padding: 1px 6px; border-radius: 4px; }
  .te.ok { color: #56d364; background: rgba(63,185,80,.15); }
  .te.ko { color: #f8918c; background: rgba(248,81,73,.15); }
  .det { color: var(--txt-faint); font-family: var(--mono); }
  .logs { margin-top: 8px; padding: 8px 10px; background: var(--bg); border-radius: 6px; font-family: var(--mono); font-size: 11.5px; color: var(--txt-dim); white-space: pre-wrap; }
</style>
