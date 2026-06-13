<script>
  // Pannello della risposta: mostra status, metriche, corpo, intestazioni ed esiti dei test.
  let { risposta, inCorso, errore, risultatiTest = [], onCapturaVar, onCreaTest } = $props();

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
    </div>

    {#if tab === "Body"}
      {#if percorsi.length > 0}
        <div class="cap-bar">
          <span class="cap-toggle" class:on={cattura} onclick={() => (cattura = !cattura)}>
            ⌖ {cattura ? "Mostra corpo" : "Cattura campi"}
          </span>
        </div>
      {/if}
      {#if cattura}
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
    {:else}
      <div class="resp-code" style="padding:8px 0">
        {#each risultatiTest as t}
          <div class="test-row">
            <span class="test-esito {t.passato ? 'ok' : 'ko'}">{t.passato ? "PASS" : "FAIL"}</span>
            <span class="test-desc">{t.descrizione}</span>
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
</style>
