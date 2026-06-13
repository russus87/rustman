<script>
  // Editor della richiesta: nome, metodo, URL, query params, intestazioni,
  // corpo, autenticazione e asserzioni. `richiesta` è un oggetto reattivo.
  let {
    richiesta,
    inCorso,
    salvabile,
    environments = [],
    ambienteAttivo = null,
    onCambiaAmbiente,
    onApriEnv,
    onInvia,
    onSalva,
  } = $props();

  let tab = $state("Body");
  const tabs = ["Params", "Headers", "Body", "Auth", "Tests", "Pre-script", "Post-script"];
  const metodi = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

  function classeMetodo(m) {
    if (m === "GET") return "get";
    if (m === "POST") return "post";
    if (m === "PUT" || m === "PATCH") return "put";
    if (m === "DELETE") return "del";
    return "";
  }

  // Intestazioni
  function aggiungiHeader() {
    richiesta.headers.push({ chiave: "", valore: "", attivo: true });
  }
  function rimuoviHeader(i) {
    richiesta.headers.splice(i, 1);
  }

  // Query params
  function aggiungiParam() {
    richiesta.params.push({ chiave: "", valore: "", attivo: true });
  }
  function rimuoviParam(i) {
    richiesta.params.splice(i, 1);
  }

  // Asserzioni (test)
  const tipiTest = ["status", "tempo", "header", "body", "json"];
  const operatori = ["==", "!=", "<", ">", "contiene"];
  function usaCampo(tipo) {
    return tipo === "header" || tipo === "json";
  }
  function aggiungiTest() {
    if (!richiesta.tests) richiesta.tests = [];
    richiesta.tests.push({ tipo: "status", operatore: "==", campo: "", atteso: "200", attivo: true });
  }
  function rimuoviTest(i) {
    richiesta.tests.splice(i, 1);
  }

  // Indenta il corpo se è JSON valido.
  function formatta() {
    try {
      richiesta.body = JSON.stringify(JSON.parse(richiesta.body), null, 2);
    } catch {
      /* corpo non JSON: nessuna modifica */
    }
  }

  // Scorciatoie: Ctrl/Cmd+Invio invia, Ctrl/Cmd+S salva.
  function suTasto(e) {
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") onInvia();
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
      e.preventDefault();
      if (salvabile) onSalva();
    }
  }
</script>

<div class="editor-col-inner" style="display:flex;flex-direction:column;min-height:0;flex:1">
  <!-- Intestazione: nome richiesta modificabile + selettore ambiente -->
  <div class="editor-head">
    <span class="m {classeMetodo(richiesta.metodo)}">{richiesta.metodo}</span>
    <input class="nome-input" placeholder="Nome richiesta" bind:value={richiesta.nome} />
    <span class="he-spacer"></span>
    <select
      class="env-select"
      value={ambienteAttivo ?? ""}
      onchange={(e) => onCambiaAmbiente(e.target.value || null)}
      title="Ambiente attivo"
    >
      <option value="">Nessun ambiente</option>
      {#each environments as e}
        <option value={e.file}>{e.environment.nome}</option>
      {/each}
    </select>
    <button class="env-gear" onclick={onApriEnv} title="Gestisci ambienti">⚙</button>
  </div>

  <!-- Barra URL -->
  <div class="urlbar">
    <div class="method-sel">
      <select class="method-select {classeMetodo(richiesta.metodo)}" bind:value={richiesta.metodo}>
        {#each metodi as m}<option value={m}>{m}</option>{/each}
      </select>
    </div>
    <div class="url-input">
      <input bind:value={richiesta.url} onkeydown={suTasto} spellcheck="false" placeholder="https://..." />
    </div>
    <div class="btn-split">
      <button class="btn btn-send main" onclick={onInvia} disabled={inCorso}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 2L11 13M22 2l-7 20-4-9-9-4z"/></svg>
        {inCorso ? "Invio..." : "Send"}
      </button>
    </div>
    <button class="btn btn-save" onclick={onSalva} disabled={!salvabile} title="Salva (Ctrl+S)">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/><path d="M17 21v-8H7v8M7 3v5h8"/></svg>Save
    </button>
  </div>

  <!-- Tab editor -->
  <div class="etabs">
    {#each tabs as t}
      <div class="etab" class:active={tab === t} onclick={() => (tab = t)}>{t}</div>
    {/each}
  </div>

  {#if tab === "Params"}
    <div class="code-wrap" style="padding:12px 14px">
      <table class="kv">
        <tbody>
          {#each richiesta.params as p, i}
            <tr>
              <td style="width:1%"><input type="checkbox" bind:checked={p.attivo} /></td>
              <td><input class="inline-input" placeholder="Chiave" bind:value={p.chiave} /></td>
              <td><input class="inline-input" placeholder="Valore" bind:value={p.valore} /></td>
              <td style="width:1%"><span class="rsp-icon" onclick={() => rimuoviParam(i)} title="Rimuovi">✕</span></td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if richiesta.params.length === 0}
        <div class="placeholder" style="height:auto;padding:14px 0"><div>Nessun parametro.</div></div>
      {/if}
      <button class="btn btn-save" style="margin-top:10px" onclick={aggiungiParam}>+ Aggiungi parametro</button>
    </div>
  {:else if tab === "Headers"}
    <div class="code-wrap" style="padding:12px 14px">
      <table class="kv">
        <tbody>
          {#each richiesta.headers as h, i}
            <tr>
              <td style="width:1%"><input type="checkbox" bind:checked={h.attivo} /></td>
              <td><input class="inline-input" placeholder="Chiave" bind:value={h.chiave} /></td>
              <td><input class="inline-input" placeholder="Valore" bind:value={h.valore} /></td>
              <td style="width:1%"><span class="rsp-icon" onclick={() => rimuoviHeader(i)} title="Rimuovi">✕</span></td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if richiesta.headers.length === 0}
        <div class="placeholder" style="height:auto;padding:14px 0"><div>Nessuna intestazione.</div></div>
      {/if}
      <button class="btn btn-save" style="margin-top:10px" onclick={aggiungiHeader}>+ Aggiungi intestazione</button>
    </div>
  {:else if tab === "Body"}
    <div class="body-bar">
      <span class="body-label">Corpo · raw</span>
      <span class="bt-spacer"></span>
      <span class="beautify" onclick={formatta} title="Indenta il JSON">Formatta</span>
    </div>
    <div class="code-wrap">
      <textarea class="code-area" bind:value={richiesta.body} spellcheck="false" placeholder={'{\n  "chiave": "valore"\n}'}></textarea>
    </div>
  {:else if tab === "Auth"}
    <div class="code-wrap" style="padding:14px 16px">
      <div class="auth-row">
        <label>Tipo</label>
        <select class="test-sel" bind:value={richiesta.auth.tipo}>
          <option value="none">Nessuna</option>
          <option value="bearer">Bearer Token</option>
          <option value="basic">Basic Auth</option>
        </select>
      </div>
      {#if richiesta.auth.tipo === "bearer"}
        <div class="auth-row"><label>Token</label><input class="inline-input" bind:value={richiesta.auth.token} placeholder="token..." /></div>
      {:else if richiesta.auth.tipo === "basic"}
        <div class="auth-row"><label>Utente</label><input class="inline-input" bind:value={richiesta.auth.utente} /></div>
        <div class="auth-row"><label>Password</label><input class="inline-input" type="password" bind:value={richiesta.auth.password} /></div>
      {/if}
    </div>
  {:else if tab === "Tests"}
    <div class="code-wrap" style="padding:12px 14px">
      <table class="kv">
        <tbody>
          {#each richiesta.tests ?? [] as t, i}
            <tr>
              <td style="width:1%"><input type="checkbox" bind:checked={t.attivo} /></td>
              <td style="width:1%"><select class="test-sel" bind:value={t.tipo}>{#each tipiTest as tt}<option value={tt}>{tt}</option>{/each}</select></td>
              <td style="width:1%"><select class="test-sel" bind:value={t.operatore}>{#each operatori as op}<option value={op}>{op}</option>{/each}</select></td>
              {#if usaCampo(t.tipo)}
                <td><input class="inline-input" placeholder={t.tipo === "json" ? "path es. data.id" : "nome header"} bind:value={t.campo} /></td>
              {/if}
              <td><input class="inline-input" placeholder="valore atteso" bind:value={t.atteso} /></td>
              <td style="width:1%"><span class="rsp-icon" onclick={() => rimuoviTest(i)} title="Rimuovi">✕</span></td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if (richiesta.tests ?? []).length === 0}
        <div class="placeholder" style="height:auto;padding:14px 0"><div>Nessuna asserzione.</div></div>
      {/if}
      <button class="btn btn-save" style="margin-top:10px" onclick={aggiungiTest}>+ Aggiungi asserzione</button>
    </div>
  {:else if tab === "Pre-script"}
    <div class="script-aiuto">Eseguito prima dell'invio. API: <code>pm.variables.set/get</code>, <code>pm.request</code>, <code>console.log</code>.</div>
    <div class="code-wrap">
      <textarea class="code-area" bind:value={richiesta.pre_script} spellcheck="false"
        placeholder={'// es: imposta una variabile\npm.variables.set("ts", Date.now());'}></textarea>
    </div>
  {:else if tab === "Post-script"}
    <div class="script-aiuto">Eseguito dopo la risposta. API: <code>pm.response.json()</code>, <code>pm.environment.set</code>, <code>pm.test(nome, fn)</code>, <code>pm.expect(x).to.equal(y)</code>.</div>
    <div class="code-wrap">
      <textarea class="code-area" bind:value={richiesta.post_script} spellcheck="false"
        placeholder={'// es: salva il token e verifica lo status\nconst d = pm.response.json();\npm.environment.set("token", d.token);\npm.test("status 200", () => pm.expect(pm.response.code).to.equal(200));'}></textarea>
    </div>
  {/if}
</div>

<style>
  /* Riga di aiuto sopra gli editor di script */
  .script-aiuto {
    padding: 8px 16px;
    border-bottom: 1px solid var(--border);
    color: var(--txt-dim);
    font-size: 11.5px;
  }
  .script-aiuto code {
    font-family: var(--mono);
    color: var(--accent-2);
  }
  /* Intestazione editor */
  .editor-head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--panel);
  }
  .he-spacer {
    flex: 1;
  }
  /* Nome richiesta modificabile */
  .nome-input {
    background: transparent;
    border: none;
    outline: none;
    color: var(--txt);
    font-size: 13px;
    width: 160px;
  }
  .nome-input:focus {
    border-bottom: 1px solid var(--accent);
  }
  /* Selettore ambiente */
  .env-select {
    appearance: none;
    -webkit-appearance: none;
    background: var(--panel-2);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 6px 12px;
    color: var(--txt);
    font-size: 12.5px;
    cursor: pointer;
    outline: none;
    margin: 7px 0;
  }
  .env-select option {
    background: var(--panel);
  }
  .env-gear {
    background: transparent;
    border: none;
    color: var(--txt-dim);
    cursor: pointer;
    font-size: 15px;
    padding: 0 8px;
  }
  .env-gear:hover {
    color: var(--txt);
  }
  /* Barra sopra l'editor del corpo */
  .body-bar {
    display: flex;
    align-items: center;
    padding: 9px 16px;
    border-bottom: 1px solid var(--border);
  }
  .body-label {
    color: var(--txt-dim);
    font-size: 12.5px;
  }
  .bt-spacer {
    flex: 1;
  }
  /* Form auth */
  .auth-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 10px;
  }
  .auth-row label {
    width: 80px;
    color: var(--txt-dim);
    font-size: 12.5px;
  }
  .auth-row .inline-input {
    flex: 1;
  }
  /* Campi comuni */
  .inline-input {
    width: 100%;
    background: var(--panel-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 9px;
    color: var(--txt);
    font-size: 12.5px;
    outline: none;
  }
  .inline-input:focus {
    border-color: var(--accent);
  }
  .test-sel {
    appearance: none;
    -webkit-appearance: none;
    background: var(--panel-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 9px;
    color: var(--txt);
    font-family: var(--mono);
    font-size: 12.5px;
    cursor: pointer;
    outline: none;
  }
  .test-sel option {
    background: var(--panel);
  }
</style>
