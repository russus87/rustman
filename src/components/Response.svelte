<script>
  // Pannello della risposta: mostra status, metriche, corpo, intestazioni ed esiti dei test.
  let { risposta, inCorso, errore, risultatiTest = [], avvisiSicurezza = [], onCapturaVar, onCreaTest, onAutoTest, onAutoSchema, onSnapshotDiff, onSnapshotAccetta } = $props();

  let tab = $state("Body"); // Body | Headers | Tests
  let cattura = $state(false); // mostra l'elenco dei campi JSON catturabili

  // Estrae i percorsi "data.items.0.id" → valore dai campi foglia del JSON.
  function estrai(val, prefix, out) {
    if (val === null || typeof val !== "object") { out.push({ path: prefix, value: val }); return out; }
    if (Array.isArray(val)) val.forEach((v, i) => estrai(v, `${prefix}.${i}`, out));
    else for (const k of Object.keys(val)) estrai(val[k], prefix ? `${prefix}.${k}` : k, out);
    return out;
  }
  const percorsi = $derived.by(() => {
    if (!risposta) return [];
    try { return estrai(JSON.parse(risposta.body), "", []).slice(0, 300); } catch { return []; }
  });

  // Vista tabella: se il body è un array di oggetti.
  let tabella = $state(false);
  let ordCol = $state(null);
  let ordAsc = $state(true);
  const datiTab = $derived.by(() => {
    if (!risposta) return null;
    try {
      const arr = JSON.parse(risposta.body);
      if (!Array.isArray(arr) || arr.length === 0 || typeof arr[0] !== "object" || arr[0] === null) return null;
      const colonne = [...new Set(arr.flatMap((o) => Object.keys(o || {})))].slice(0, 20);
      let righe = arr.slice(0, 500);
      if (ordCol) {
        righe = [...righe].sort((a, b) => {
          const x = a?.[ordCol], y = b?.[ordCol];
          return (x > y ? 1 : x < y ? -1 : 0) * (ordAsc ? 1 : -1);
        });
      }
      return { colonne, righe };
    } catch { return null; }
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

  // Prova a formattare il corpo come JSON indentato; altrimenti lo lascia grezzo.
  function formattaBody(testo) {
    try {
      return JSON.stringify(JSON.parse(testo), null, 2);
    } catch {
      return testo;
    }
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
      {#if percorsi.length > 0 || datiTab}
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
        </div>
      {/if}
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
        <div class="resp-code">
          <div class="code">
            <div class="lines">{formattaBody(risposta.body)}</div>
          </div>
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
  .cap-bar { padding: 6px 12px; border-bottom: 1px solid var(--border); }
  .cap-toggle { cursor: pointer; font-size: 12px; color: var(--txt-dim); padding: 3px 8px; border-radius: 6px; border: 1px solid var(--border); }
  .cap-toggle:hover, .cap-toggle.on { color: var(--txt); background: var(--panel-3); }
  .cap-tab td { padding: 4px 10px; font-family: var(--mono); font-size: 12px; border-bottom: 1px solid var(--border); }
  .cap-path { color: var(--accent-2); white-space: nowrap; }
  .cap-val { color: var(--txt-dim); max-width: 240px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cap-act { white-space: nowrap; text-align: right; }
  .cap-b { cursor: pointer; color: var(--txt-faint); margin-left: 8px; }
  .cap-b:hover { color: var(--accent); }
  .tab-grid th { position: sticky; top: 0; background: var(--panel-2); cursor: pointer; white-space: nowrap; color: var(--txt-dim); font-weight: 600; padding: 6px 10px; border-bottom: 1px solid var(--border); user-select: none; }
  .tab-grid th:hover { color: var(--txt); }
  .tab-grid td { padding: 4px 10px; border-bottom: 1px solid var(--border); max-width: 220px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: var(--mono); font-size: 12px; }
</style>
