<script>
  // Pannello della risposta: mostra status, metriche, corpo, intestazioni ed esiti dei test.
  let { risposta, inCorso, errore, risultatiTest = [] } = $props();

  let tab = $state("Body"); // Body | Headers | Tests

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
      <div class="resp-code">
        <div class="code">
          <div class="lines">{formattaBody(risposta.body)}</div>
        </div>
      </div>
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
</style>
