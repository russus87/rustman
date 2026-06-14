<script>
  // Editor della richiesta: nome, metodo, URL, query params, intestazioni,
  // corpo, autenticazione e asserzioni. `richiesta` è un oggetto reattivo.
  let {
    richiesta,
    inCorso,
    salvabile,
    environments = [],
    ambienteAttivo = null,
    variabili = null,
    onCambiaAmbiente,
    onApriEnv,
    onInvia,
    onSalva,
    onCopiaCodice,
    onOttieniToken,
  } = $props();

  let lingCodice = $state("curl");

  let tab = $state("Body");
  const tabs = ["Params", "Headers", "Body", "Auth", "Rete", "Tests", "Pre-script", "Post-script"];
  const metodi = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

  // Anteprima dei {{segnaposto}} nell'URL: variabili d'ambiente risolte,
  // dinamiche ($...) mostrate come marcatore (verranno generate all'invio).
  function risolviAnteprima(testo) {
    return (testo || "").replace(/\{\{\s*([^}]+?)\s*\}\}/g, (m, nome) => {
      if (nome.startsWith("$")) return `‹${nome}›`;
      const v = variabili?.[nome];
      return v !== undefined ? v : m;
    });
  }
  const urlRisolto = $derived(risolviAnteprima(richiesta.url));

  // Autocomplete dei {{...}} nell'URL.
  let urlEl = $state(null);
  let acAperto = $state(false);
  let acIndice = $state(0);
  const DINAMICHE = ["$timestamp", "$isoTimestamp", "$randomUUID", "$randomInt", "$randomFloat",
    "$randomBool", "$name", "$email", "$company", "$city", "$lorem", "$phone"];

  function contestoVar(valore, caret) {
    const prima = valore.slice(0, caret);
    const apri = prima.lastIndexOf("{{");
    if (apri < 0 || prima.slice(apri).includes("}}")) return null;
    const partial = prima.slice(apri + 2);
    if (/[\s{}]/.test(partial)) return null;
    return { partial, apri };
  }
  const acSuggerimenti = $derived.by(() => {
    if (!acAperto || !urlEl) return [];
    const ctx = contestoVar(richiesta.url, urlEl.selectionStart ?? richiesta.url.length);
    if (!ctx) return [];
    const tutte = [...Object.keys(variabili || {}), ...DINAMICHE];
    return tutte.filter((v) => v.toLowerCase().startsWith(ctx.partial.toLowerCase())).slice(0, 8);
  });
  function suInputUrl() {
    const ctx = contestoVar(richiesta.url, urlEl?.selectionStart ?? 0);
    acAperto = !!ctx;
    acIndice = 0;
  }
  function inserisciVar(nome) {
    const caret = urlEl.selectionStart;
    const ctx = contestoVar(richiesta.url, caret);
    if (!ctx) return;
    const before = richiesta.url.slice(0, ctx.apri);
    const after = richiesta.url.slice(caret);
    richiesta.url = `${before}{{${nome}}}${after}`;
    acAperto = false;
    const pos = before.length + nome.length + 4;
    requestAnimationFrame(() => { urlEl.focus(); urlEl.setSelectionRange(pos, pos); });
  }
  function suTastoUrl(e) {
    if (acAperto && acSuggerimenti.length) {
      if (e.key === "ArrowDown") { e.preventDefault(); acIndice = Math.min(acIndice + 1, acSuggerimenti.length - 1); return; }
      if (e.key === "ArrowUp") { e.preventDefault(); acIndice = Math.max(acIndice - 1, 0); return; }
      if (e.key === "Enter" || e.key === "Tab") { e.preventDefault(); inserisciVar(acSuggerimenti[acIndice]); return; }
      if (e.key === "Escape") { acAperto = false; return; }
    }
    suTasto(e);
  }

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
  const tipiTest = ["status", "tempo", "header", "body", "json", "schema", "snapshot"];
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

  // Corpo: modalità e campi del form (form-data / urlencoded).
  // Garantisce i valori di default su richieste vecchie o create al volo.
  $effect(() => {
    if (richiesta && richiesta.body_mode == null) richiesta.body_mode = "raw";
    if (richiesta && !Array.isArray(richiesta.form)) richiesta.form = [];
    if (richiesta && richiesta.auth && richiesta.auth.oauth2 == null) {
      richiesta.auth.oauth2 = {
        grant_type: "client_credentials", token_url: "", auth_url: "",
        client_id: "", client_secret: "", username: "", password: "",
        scope: "", access_token: "",
      };
    }
    if (richiesta && richiesta.impostazioni == null) {
      richiesta.impostazioni = { timeout_ms: 0, segui_redirect: true, verifica_tls: true, retry_429: 0 };
    }
    if (richiesta && !Array.isArray(richiesta.tags)) richiesta.tags = [];
  });

  // OAuth2: chiede il token al server di autorizzazione e lo salva nella richiesta.
  let tokenInCorso = $state(false);
  async function ottieniToken() {
    if (!onOttieniToken) return;
    tokenInCorso = true;
    try {
      const token = await onOttieniToken($state.snapshot(richiesta.auth));
      if (token) richiesta.auth.oauth2.access_token = token;
    } finally {
      tokenInCorso = false;
    }
  }
  function aggiungiCampoForm() {
    if (!richiesta.form) richiesta.form = [];
    richiesta.form.push({ chiave: "", valore: "", tipo: "text", file: "", attivo: true });
  }
  function rimuoviCampoForm(i) {
    richiesta.form.splice(i, 1);
  }
  // Sceglie un file dal disco (solo desktop): serve il percorso per il multipart.
  async function sfogliaFile(i) {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const scelta = await open({ multiple: false, title: "Scegli un file da allegare" });
      if (typeof scelta === "string") richiesta.form[i].file = scelta;
    } catch {
      /* su web la scelta del percorso file non è disponibile */
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
    <input class="tags-input" placeholder="🏷 tag…" title="Tag separati da virgola"
      value={(richiesta.tags || []).join(", ")}
      oninput={(e) => (richiesta.tags = e.target.value.split(",").map((s) => s.trim()).filter(Boolean))} />
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
    <div class="url-input" style="position:relative">
      <input bind:this={urlEl} bind:value={richiesta.url} oninput={suInputUrl} onkeydown={suTastoUrl}
        onblur={() => requestAnimationFrame(() => (acAperto = false))} spellcheck="false" placeholder="https://..." />
      {#if acAperto && acSuggerimenti.length}
        <div class="ac-pop">
          {#each acSuggerimenti as s, i}
            <div class="ac-item" class:active={i === acIndice} onmousedown={(e) => { e.preventDefault(); inserisciVar(s); }}>
              <span class="ac-brace">{"{{"}</span>{s}<span class="ac-brace">{"}}"}</span>
              {#if s.startsWith("$")}<span class="ac-tag">din</span>{/if}
            </div>
          {/each}
        </div>
      {/if}
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
    <select class="test-sel" bind:value={lingCodice} title="Linguaggio dello snippet">
      <option value="curl">cURL</option>
      <option value="fetch">fetch</option>
      <option value="python">Python</option>
    </select>
    <button class="btn btn-save" onclick={() => onCopiaCodice?.(richiesta, lingCodice)} title="Copia come codice">Copia</button>
  </div>
  {#if richiesta.url.includes("{{")}
    <div class="url-preview" title="URL con le variabili risolte">→ {urlRisolto}</div>
  {/if}

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
      <select class="test-sel" bind:value={richiesta.body_mode}>
        <option value="raw">raw</option>
        <option value="form-data">form-data</option>
        <option value="x-www-form-urlencoded">x-www-form-urlencoded</option>
      </select>
      <span class="bt-spacer"></span>
      {#if (richiesta.body_mode ?? "raw") === "raw"}
        <span class="beautify" onclick={formatta} title="Indenta il JSON">Formatta</span>
      {/if}
    </div>
    {#if (richiesta.body_mode ?? "raw") === "raw"}
      <div class="code-wrap">
        <textarea class="code-area" bind:value={richiesta.body} spellcheck="false" placeholder={'{\n  "chiave": "valore"\n}'}></textarea>
      </div>
    {:else}
      <div class="code-wrap" style="padding:12px 14px">
        <table class="kv">
          <tbody>
            {#each richiesta.form ?? [] as c, i}
              <tr>
                <td style="width:1%"><input type="checkbox" bind:checked={c.attivo} /></td>
                <td><input class="inline-input" placeholder="chiave" bind:value={c.chiave} /></td>
                {#if richiesta.body_mode === "form-data"}
                  <td style="width:1%"><select class="test-sel" bind:value={c.tipo}><option value="text">text</option><option value="file">file</option></select></td>
                {/if}
                {#if richiesta.body_mode === "form-data" && c.tipo === "file"}
                  <td><div class="file-cell"><input class="inline-input" placeholder="percorso file…" bind:value={c.file} /><button class="mini-b" onclick={() => sfogliaFile(i)}>Sfoglia</button></div></td>
                {:else}
                  <td><input class="inline-input" placeholder="valore" bind:value={c.valore} /></td>
                {/if}
                <td style="width:1%"><span class="rsp-icon" onclick={() => rimuoviCampoForm(i)} title="Rimuovi">✕</span></td>
              </tr>
            {/each}
          </tbody>
        </table>
        {#if (richiesta.form ?? []).length === 0}
          <div class="placeholder" style="height:auto;padding:14px 0"><div>Nessun campo.</div></div>
        {/if}
        <button class="btn btn-save" style="margin-top:10px" onclick={aggiungiCampoForm}>+ Aggiungi campo</button>
        {#if richiesta.body_mode === "form-data"}
          <div class="script-aiuto" style="border:none;padding:8px 0 0">I file vengono letti dal disco all'invio (solo app desktop).</div>
        {/if}
      </div>
    {/if}
  {:else if tab === "Auth"}
    <div class="code-wrap" style="padding:14px 16px">
      <div class="auth-row">
        <label>Tipo</label>
        <select class="test-sel" bind:value={richiesta.auth.tipo}>
          <option value="none">Nessuna</option>
          <option value="bearer">Bearer Token</option>
          <option value="basic">Basic Auth</option>
          <option value="oauth2">OAuth 2.0</option>
        </select>
      </div>
      {#if richiesta.auth.tipo === "bearer"}
        <div class="auth-row"><label>Token</label><input class="inline-input" bind:value={richiesta.auth.token} placeholder="token..." /></div>
      {:else if richiesta.auth.tipo === "basic"}
        <div class="auth-row"><label>Utente</label><input class="inline-input" bind:value={richiesta.auth.utente} /></div>
        <div class="auth-row"><label>Password</label><input class="inline-input" type="password" bind:value={richiesta.auth.password} /></div>
      {:else if richiesta.auth.tipo === "oauth2"}
        <div class="auth-row">
          <label>Grant</label>
          <select class="test-sel" bind:value={richiesta.auth.oauth2.grant_type}>
            <option value="client_credentials">Client Credentials</option>
            <option value="password">Password</option>
          </select>
        </div>
        <div class="auth-row"><label>Token URL</label><input class="inline-input" bind:value={richiesta.auth.oauth2.token_url} placeholder="https://.../oauth/token" /></div>
        <div class="auth-row"><label>Client ID</label><input class="inline-input" bind:value={richiesta.auth.oauth2.client_id} /></div>
        <div class="auth-row"><label>Client Secret</label><input class="inline-input" type="password" bind:value={richiesta.auth.oauth2.client_secret} /></div>
        {#if richiesta.auth.oauth2.grant_type === "password"}
          <div class="auth-row"><label>Username</label><input class="inline-input" bind:value={richiesta.auth.oauth2.username} /></div>
          <div class="auth-row"><label>Password</label><input class="inline-input" type="password" bind:value={richiesta.auth.oauth2.password} /></div>
        {/if}
        <div class="auth-row"><label>Scope</label><input class="inline-input" bind:value={richiesta.auth.oauth2.scope} placeholder="(facoltativo)" /></div>
        <div class="auth-row">
          <label>Access Token</label>
          <input class="inline-input" type="password" bind:value={richiesta.auth.oauth2.access_token} placeholder="ottenuto o incollato" />
        </div>
        <div class="auth-row">
          <button class="btn btn-save" onclick={ottieniToken} disabled={tokenInCorso}>{tokenInCorso ? "Richiesta..." : "Ottieni token"}</button>
        </div>
      {/if}
    </div>
  {:else if tab === "Rete"}
    <div class="code-wrap" style="padding:14px 16px">
      {#if richiesta.impostazioni}
        <div class="auth-row"><label>Timeout (ms)</label><input class="inline-input" type="number" min="0" placeholder="0 = nessuno" bind:value={richiesta.impostazioni.timeout_ms} /></div>
        <div class="auth-row"><label>Retry su 429</label><input class="inline-input" type="number" min="0" max="10" title="Ritentativi su 429 rispettando Retry-After" bind:value={richiesta.impostazioni.retry_429} /></div>
        <div class="auth-row"><label><input type="checkbox" bind:checked={richiesta.impostazioni.segui_redirect} /> Segui i redirect</label></div>
        <div class="auth-row"><label><input type="checkbox" bind:checked={richiesta.impostazioni.verifica_tls} /> Verifica il certificato TLS</label></div>
        <div class="script-aiuto" style="border:none;padding:8px 0 0">I cookie (Set-Cookie) sono gestiti automaticamente come sessione tra le richieste.</div>
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
  /* Campo file del form-data: input percorso + pulsante "Sfoglia". */
  .file-cell {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .mini-b {
    background: var(--panel-3);
    color: var(--txt);
    border: 1px solid var(--border-2);
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 12px;
    cursor: pointer;
    white-space: nowrap;
  }
  .mini-b:hover {
    background: #22222e;
  }
  .tags-input {
    width: 140px; background: var(--panel-2); border: 1px solid var(--border);
    border-radius: 6px; padding: 5px 9px; color: var(--txt-dim); font-size: 12px; outline: none;
  }
  .tags-input:focus { border-color: var(--accent); color: var(--txt); }
  .ac-pop {
    position: absolute; top: 100%; left: 0; right: 0; z-index: 40; margin-top: 2px;
    background: var(--panel); border: 1px solid var(--border-2); border-radius: 8px;
    box-shadow: 0 12px 32px rgba(0,0,0,.45); overflow: hidden; max-height: 220px; overflow-y: auto;
  }
  .ac-item { display: flex; align-items: center; gap: 4px; padding: 7px 10px; cursor: pointer; font-family: var(--mono); font-size: 12.5px; color: var(--txt); }
  .ac-item.active { background: rgba(124,92,255,.18); }
  .ac-brace { color: var(--txt-faint); }
  .ac-tag { margin-left: auto; font-size: 10px; color: var(--accent-2); background: var(--panel-3); padding: 1px 5px; border-radius: 4px; }
  .url-preview {
    padding: 4px 16px 6px;
    font-family: var(--mono);
    font-size: 11.5px;
    color: var(--txt-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
