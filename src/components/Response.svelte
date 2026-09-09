<script>
  // Pannello della risposta: mostra status, metriche, corpo, intestazioni ed esiti dei test.
  let { risposta, inCorso, errore, risultatiTest = [], avvisiSicurezza = [], onCapturaVar, onCreaTest, onAutoTest, onAutoSchema, onSnapshotDiff, onSnapshotAccetta, onSalvaEsempio } = $props();

  import CodeEditor from "./CodeEditor.svelte";
  import { trasformaJson } from "../lib/json-fmt.js";
  import { fasiDaJson, fasiDaTesto, riepilogo, fmtMs } from "../lib/fasi.js";

  let tab = $state("Body"); // Body | Headers | Fasi | Tests
  let cattura = $state(false); // mostra l'elenco dei campi JSON catturabili
  let fasiPerDurata = $state(false); // ordina le fasi dalla più lenta

  // Oltre questa dimensione il corpo non viene interpretato per "cattura
  // campi" e "tabella": richiederebbero di attraversare tutto il JSON, e su
  // una risposta da decine di MB non varrebbe comunque la pena.
  const SOGLIA_ANALISI = 4 * 1024 * 1024;
  // Sotto questa soglia indentare è istantaneo e si fa qui; sopra si passa
  // dal worker, per non bloccare la finestra.
  const SOGLIA_FMT_SYNC = 256 * 1024;
  // Quanti campi mostrare in "cattura campi" e quante righe/colonne in tabella.
  const MAX_PERCORSI = 300;
  const MAX_RIGHE_TAB = 500;
  const CAMPIONE_COLONNE = 200;

  // Il corpo viene interpretato UNA volta sola. Prima "cattura campi" e
  // "tabella" facevano ciascuno il proprio JSON.parse dell'intero corpo, e la
  // tabella lo rifaceva a ogni click sull'intestazione per riordinare: su una
  // risposta da 10 MB erano centinaia di millisecondi a ogni render.
  const radice = $derived.by(() => {
    const b = risposta?.body;
    if (!b || b.length > SOGLIA_ANALISI) return undefined;
    try { return JSON.parse(b); } catch { return undefined; }
  });
  const troppoGrande = $derived(!!risposta?.body && risposta.body.length > SOGLIA_ANALISI);

  // Percorsi "data.items.0.id" → valore delle foglie, fermandosi a `max`:
  // prima l'elenco veniva costruito per intero (oltre mezzo milione di voci su
  // una risposta grande) e solo dopo tagliato a 300.
  function estrai(val, prefix, out, max) {
    if (out.length >= max) return out;
    if (val === null || typeof val !== "object") { out.push({ path: prefix, value: val }); return out; }
    if (Array.isArray(val)) {
      for (let i = 0; i < val.length && out.length < max; i++) estrai(val[i], `${prefix}.${i}`, out, max);
    } else {
      for (const k of Object.keys(val)) {
        if (out.length >= max) break;
        estrai(val[k], prefix ? `${prefix}.${k}` : k, out, max);
      }
    }
    return out;
  }
  const percorsi = $derived(radice === undefined ? [] : estrai(radice, "", [], MAX_PERCORSI));

  // ---- Fasi dichiarate dal servizio ----
  // Alcune API raccontano nel corpo quanto è durata ogni tappa del lavoro
  // ("Step: LoadFontStep completed in 349.9064 ms"). Quando ci sono si può dire
  // quanto pesa ciascuna sul totale; quando non ci sono qui non cambia nulla.
  const fasi = $derived.by(() => {
    const b = risposta?.body;
    if (!b || troppoGrande) return [];
    return radice === undefined ? fasiDaTesto(b) : fasiDaJson(radice);
  });
  const rifasi = $derived(riepilogo(fasi, risposta?.tempo_ms ?? 0));
  const fasiMostrate = $derived(
    fasiPerDurata ? [...rifasi.righe].sort((a, b) => b.ms - a.ms) : rifasi.righe,
  );
  const fasiMax = $derived(rifasi.piuLenta?.ms || 1);
  // Se la risposta nuova non dichiara fasi il tab sparisce: si torna al corpo.
  $effect(() => {
    if (tab === "Fasi" && fasi.length === 0) tab = "Body";
  });

  // Vista tabella: se il body è un array di oggetti.
  let tabella = $state(false);
  let ordCol = $state(null);
  let ordAsc = $state(true);
  const datiTab = $derived.by(() => {
    const arr = radice;
    if (!Array.isArray(arr) || arr.length === 0 || typeof arr[0] !== "object" || arr[0] === null) return null;
    // Le colonne si ricavano da un campione: scorrere decine di migliaia di
    // elementi solo per comporre l'intestazione non aggiunge nulla.
    const colonne = [...new Set(arr.slice(0, CAMPIONE_COLONNE).flatMap((o) => Object.keys(o || {})))].slice(0, 20);
    let righe = arr.slice(0, MAX_RIGHE_TAB);
    if (ordCol) {
      righe = [...righe].sort((a, b) => {
        const x = a?.[ordCol], y = b?.[ordCol];
        return (x > y ? 1 : x < y ? -1 : 0) * (ordAsc ? 1 : -1);
      });
    }
    return { colonne, righe, totale: arr.length };
  });
  function ordina(c) {
    if (ordCol === c) ordAsc = !ordAsc; else { ordCol = c; ordAsc = true; }
  }
  function cella(v) {
    if (v === null || v === undefined) return "";
    return typeof v === "object" ? JSON.stringify(v) : String(v);
  }

  // Quanti test sono passati sul totale.
  const passati = $derived(risultatiTest.filter((t) => t.passato).length);

  // ---- Corpo mostrato nell'editor ----
  // L'indentazione di un corpo grande costa quanto la sua dimensione: sotto
  // SOGLIA_FMT_SYNC si fa qui (istantanea, nessun lampeggio), sopra la fa il
  // worker mentre intanto si vede il corpo grezzo.
  let bodyMostrato = $state("");
  let formattando = $state(false);
  let statoBody = $state({ righe: 0, caratteri: 0, ricca: true, aCapo: true, rigaLunga: false });
  let aCapoBody = $state(true);

  $effect(() => {
    const b = risposta?.body ?? "";
    if (!b) { bodyMostrato = ""; formattando = false; return; }
    if (b.length <= SOGLIA_FMT_SYNC) {
      try { bodyMostrato = JSON.stringify(JSON.parse(b), null, 2); }
      catch { bodyMostrato = b; }
      formattando = false;
      return;
    }
    bodyMostrato = b;
    formattando = true;
    let vivo = true;
    trasformaJson(b, "formatta")
      .then((t) => { if (vivo) bodyMostrato = t; })
      .catch(() => { /* non è JSON: resta il corpo grezzo */ })
      .finally(() => { if (vivo) formattando = false; });
    return () => { vivo = false; };
  });

  function numero(n) {
    return n.toLocaleString("it-IT");
  }
  function pesoTesto(caratteri) {
    if (caratteri < 1024) return `${caratteri} car.`;
    if (caratteri < 1024 * 1024) return `${(caratteri / 1024).toFixed(1)} K car.`;
    return `${(caratteri / 1048576).toFixed(2)} M car.`;
  }

  // Converte i byte in una stringa leggibile (B / KB).
  function dimensione(byte) {
    if (byte < 1024) return `${byte} B`;
    return `${(byte / 1024).toFixed(2)} KB`;
  }
</script>

<div class="response">
  <div class="resp-head">
    <span class="ttl">Response</span>
    <span class="resp-spacer"></span>
    {#if risposta}
      <span class="badge {risposta.status < 400 ? 'ok' : 'err'}">{risposta.status} {risposta.status_text}</span>
      <span class="resp-meta">{risposta.tempo_ms} ms</span>
      <span class="resp-meta">{dimensione(risposta.dimensione)}</span>
      <span class="gen-test" title="Genera asserzioni dalla risposta" onclick={() => onAutoTest?.(risposta)}>＋ test</span>
      <span class="gen-test" title="Crea asserzione schema dalla risposta" onclick={() => onAutoSchema?.(risposta)}>＋ schema</span>
      <span class="gen-test" title="Salva come esempio di risposta" onclick={() => onSalvaEsempio?.(risposta)}>＋ esempio</span>
    {/if}
  </div>

  {#if inCorso}
    <div class="placeholder"><div class="big">Invio in corso…</div></div>
  {:else if errore}
    <div class="err-box">{errore}</div>
  {:else if !risposta}
    <div class="placeholder">
      <div class="big">Nessuna risposta</div>
      <div>Premi <b>Send</b> per inviare la richiesta.</div>
    </div>
  {:else}
    <div class="resp-tabs">
      <div class="rsp-tab" class:active={tab === "Body"} onclick={() => (tab = "Body")}>Body</div>
      <div class="rsp-tab" class:active={tab === "Headers"} onclick={() => (tab = "Headers")}>
        Headers <span class="cnt">({risposta.headers.length})</span>
      </div>
      {#if fasi.length > 0}
        <div class="rsp-tab" class:active={tab === "Fasi"} onclick={() => (tab = "Fasi")}>
          Fasi <span class="cnt">({fasi.length})</span>
        </div>
      {/if}
      {#if risultatiTest.length > 0}
        <div class="rsp-tab" class:active={tab === "Tests"} onclick={() => (tab = "Tests")}>
          Tests <span class="cnt">({passati}/{risultatiTest.length})</span>
        </div>
      {/if}
      {#if avvisiSicurezza.length > 0}
        <div class="rsp-tab" class:active={tab === "Sicurezza"} onclick={() => (tab = "Sicurezza")}>
          Sicurezza <span class="cnt">({avvisiSicurezza.length})</span>
        </div>
      {/if}
    </div>

    {#if tab === "Body"}
      <div class="cap-bar">
        {#if percorsi.length > 0}
          <span class="cap-toggle" class:on={cattura} onclick={() => { cattura = !cattura; if (cattura) tabella = false; }}>
            ⌖ {cattura ? "Mostra corpo" : "Cattura campi"}
          </span>
        {/if}
        {#if datiTab}
          <span class="cap-toggle" class:on={tabella} onclick={() => { tabella = !tabella; if (tabella) cattura = false; }}>
            ▦ {tabella ? "Mostra corpo" : "Tabella"}
          </span>
        {/if}
        {#if !cattura && !tabella}
          <span class="body-stato">{numero(statoBody.righe)} righe · {pesoTesto(statoBody.caratteri)}</span>
          {#if formattando}<span class="body-nota">indento…</span>
          {:else if troppoGrande}
            <span class="body-nota" title="Sopra i 4 MB il corpo non viene analizzato per cattura campi e tabella: servirebbe attraversarlo tutto a ogni render.">corpo grande · analisi campi off</span>
          {/if}
          {#if !statoBody.ricca}<span class="body-nota">modalità leggera</span>{/if}
          {#if statoBody.rigaLunga}<span class="body-nota">riga unica · a capo off</span>{/if}
          <span class="cap-sp"></span>
          <label class="body-chk" title="Manda a capo le righe lunghe">
            <input type="checkbox" bind:checked={aCapoBody} /> a capo
          </label>
        {/if}
      </div>
      {#if tabella && datiTab}
        <div class="resp-code" style="overflow:auto">
          <table class="kv tab-grid">
            <thead><tr>{#each datiTab.colonne as c}<th onclick={() => ordina(c)}>{c}{ordCol === c ? (ordAsc ? " ▲" : " ▼") : ""}</th>{/each}</tr></thead>
            <tbody>
              {#each datiTab.righe as r}
                <tr>{#each datiTab.colonne as c}<td title={cella(r?.[c])}>{cella(r?.[c])}</td>{/each}</tr>
              {/each}
            </tbody>
          </table>
          {#if datiTab.totale > datiTab.righe.length}
            <div class="tab-nota">Prime {numero(datiTab.righe.length)} righe di {numero(datiTab.totale)}.</div>
          {/if}
        </div>
      {:else if cattura}
        <div class="resp-code">
          <table class="kv cap-tab">
            <tbody>
              {#each percorsi as p}
                <tr>
                  <td class="cap-path">{p.path}</td>
                  <td class="cap-val" title={String(p.value)}>{String(p.value)}</td>
                  <td class="cap-act">
                    <span class="cap-b" title="Salva come variabile d'ambiente" onclick={() => onCapturaVar?.(p.path, String(p.value))}>→ var</span>
                    <span class="cap-b" title="Crea un'asserzione json" onclick={() => onCreaTest?.(p.path, String(p.value))}>→ test</span>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else}
        <div class="resp-code" style="display:flex;overflow:hidden">
          <CodeEditor
            chiave={risposta}
            valore={bodyMostrato}
            onStato={(s) => (statoBody = s)}
            aCapo={aCapoBody}
            soloLettura
          />
        </div>
      {/if}
    {:else if tab === "Headers"}
      <div class="resp-code">
        <table class="kv">
          <tbody>
            {#each risposta.headers as h}
              <tr><td class="k">{h.chiave}</td><td class="v">{h.valore}</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else if tab === "Fasi"}
      <div class="cap-bar">
        <span class="body-stato">somma delle fasi <b>{fmtMs(rifasi.somma)}</b></span>
        <span class="body-stato">risposta <b>{fmtMs(risposta.tempo_ms)}</b></span>
        {#if rifasi.altro > 0 && risposta.tempo_ms > 0}
          <span class="body-stato" title="Tempo di risposta che nessuna fase spiega: rete, attesa in coda, serializzazione.">
            altro <b>{fmtMs(rifasi.altro)}</b> ({(rifasi.altro / risposta.tempo_ms * 100).toFixed(1)}%)
          </span>
        {/if}
        <span class="cap-sp"></span>
        <span class="cap-toggle" class:on={fasiPerDurata} onclick={() => (fasiPerDurata = !fasiPerDurata)}>
          {fasiPerDurata ? "≡ in ordine" : "↓ per durata"}
        </span>
      </div>
      <div class="resp-code" style="overflow:auto">
        <table class="kv fasi-tab">
          <tbody>
            {#each fasiMostrate as f}
              <tr>
                <td class="f-nome" title={f.nome}>{f.nome}</td>
                <td class="f-barra">
                  <div class="f-b" class:top={f === rifasi.piuLenta} style="width:{(f.ms / fasiMax) * 100}%"></div>
                </td>
                <td class="f-ms">{fmtMs(f.ms)}</td>
                <td class="f-quota" class:top={f === rifasi.piuLenta}>{f.quota.toFixed(1)}%</td>
              </tr>
            {/each}
          </tbody>
        </table>
        <div class="tab-nota">
          Tempi dichiarati dal servizio nel corpo della risposta; la percentuale è
          sulla somma delle fasi.
        </div>
      </div>
    {:else if tab === "Sicurezza"}
      <div class="resp-code" style="padding:8px 0">
        {#each avvisiSicurezza as a}
          <div class="test-row">
            <span class="sec-liv {a.livello}">{a.livello}</span>
            <span class="test-desc">{a.titolo}</span>
            <span class="test-det">{a.dettaglio}</span>
          </div>
        {/each}
      </div>
    {:else}
      <div class="resp-code" style="padding:8px 0">
        {#each risultatiTest as t}
          <div class="test-row">
            <span class="test-esito {t.passato ? 'ok' : 'ko'}">{t.passato ? "PASS" : "FAIL"}</span>
            <span class="test-desc">{t.descrizione}</span>
            {#if t.descrizione === "snapshot" && !t.passato}
              <span class="snap-act" onclick={() => onSnapshotDiff?.()}>diff</span>
              <span class="snap-act ok" onclick={() => onSnapshotAccetta?.()}>✓ accetta</span>
            {/if}
            <span class="test-det">{t.dettaglio}</span>
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  /* Righe degli esiti dei test */
  .test-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border);
    font-size: 12.5px;
  }
  .test-esito {
    font-family: var(--mono);
    font-weight: 700;
    font-size: 11px;
    padding: 2px 7px;
    border-radius: 5px;
  }
  .test-esito.ok {
    color: #56d364;
    background: rgba(63, 185, 80, 0.15);
  }
  .test-esito.ko {
    color: #f8918c;
    background: rgba(248, 81, 73, 0.15);
  }
  .sec-liv { font-family: var(--mono); font-weight: 700; font-size: 10.5px; text-transform: uppercase; padding: 2px 7px; border-radius: 5px; }
  .sec-liv.alto { color: #f8918c; background: rgba(248,81,73,.15); }
  .sec-liv.medio { color: #e2b340; background: rgba(226,179,64,.15); }
  .sec-liv.info { color: #9aa7b8; background: var(--panel-3); }
  .gen-test { cursor: pointer; font-size: 11.5px; color: var(--txt-faint); border: 1px solid var(--border); border-radius: 6px; padding: 2px 8px; }
  .gen-test:hover { color: var(--accent); border-color: var(--accent); }
  .snap-act { cursor: pointer; font-size: 11px; color: var(--txt-faint); border: 1px solid var(--border); border-radius: 5px; padding: 1px 7px; }
  .snap-act:hover { color: var(--accent); border-color: var(--accent); }
  .snap-act.ok:hover { color: var(--green); border-color: var(--green); }
  .test-desc {
    font-family: var(--mono);
    color: var(--txt);
  }
  .test-det {
    margin-left: auto;
    color: var(--txt-faint);
    font-family: var(--mono);
    font-size: 11.5px;
  }
  /* Cattura campi dal JSON */
  .cap-bar {
    display: flex; flex-wrap: wrap; align-items: center; gap: 6px 10px;
    padding: 6px 12px; border-bottom: 1px solid var(--border);
  }
  .cap-sp { flex: 1; }
  .body-stato { color: var(--txt-faint); font-family: var(--mono); font-size: 11.5px; white-space: nowrap; }
  .body-nota {
    color: var(--orange); background: var(--panel-3); white-space: nowrap;
    border-radius: 4px; padding: 1px 6px; font-size: 10.5px;
  }
  .body-chk {
    display: flex; align-items: center; gap: 5px; white-space: nowrap;
    color: var(--txt-dim); font-size: 11.5px; cursor: pointer;
  }
  .tab-nota { padding: 8px 12px; color: var(--txt-faint); font-size: 11.5px; }
  .cap-toggle { cursor: pointer; font-size: 12px; color: var(--txt-dim); padding: 3px 8px; border-radius: 6px; border: 1px solid var(--border); }
  .cap-toggle:hover, .cap-toggle.on { color: var(--txt); background: var(--panel-3); }
  .cap-tab td { padding: 4px 10px; font-family: var(--mono); font-size: 12px; border-bottom: 1px solid var(--border); }
  .cap-path { color: var(--accent-2); white-space: nowrap; }
  .cap-val { color: var(--txt-dim); max-width: 240px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cap-act { white-space: nowrap; text-align: right; }
  .cap-b { cursor: pointer; color: var(--txt-faint); margin-left: 8px; }
  .cap-b:hover { color: var(--accent); }
  /* Fasi dichiarate dal servizio */
  .fasi-tab { width: 100%; }
  .fasi-tab td {
    padding: 5px 10px; border-bottom: 1px solid var(--border);
    font-family: var(--mono); font-size: 12px; vertical-align: middle;
  }
  .f-nome { color: var(--txt); max-width: 260px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .f-barra { width: 100%; }
  .f-b {
    height: 9px; border-radius: 3px; min-width: 2px;
    background: linear-gradient(90deg, var(--accent-2), var(--accent));
    opacity: .55;
  }
  .f-b.top { opacity: 1; }
  .f-ms { color: var(--txt-dim); text-align: right; white-space: nowrap; }
  .f-quota { color: var(--txt-faint); text-align: right; white-space: nowrap; }
  .f-quota.top { color: var(--txt); font-weight: 600; }
  .tab-grid th { position: sticky; top: 0; background: var(--panel-2); cursor: pointer; white-space: nowrap; color: var(--txt-dim); font-weight: 600; padding: 6px 10px; border-bottom: 1px solid var(--border); user-select: none; }
  .tab-grid th:hover { color: var(--txt); }
  .tab-grid td { padding: 4px 10px; border-bottom: 1px solid var(--border); max-width: 220px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: var(--mono); font-size: 12px; }
</style>
