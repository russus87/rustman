<script>
  // Console GraphQL: editor query + variabili, esecuzione e explorer dello schema
  // (via introspezione). Riusa l'invio HTTP esistente (POST {query, variables}).
  import * as api from "../lib/api.js";
  let { tab, variabili = null } = $props();

  let inCorso = $state(false);
  let risposta = $state(null);
  let errore = $state(null);
  let schema = $state(null); // [{ name, fields: [{name, type}] }]
  let cerca = $state("");

  function reqBase(query, variables) {
    return {
      nome: "GraphQL", metodo: "POST", url: tab.url || "",
      headers: [{ chiave: "Content-Type", valore: "application/json", attivo: true }],
      params: [], auth: { tipo: "none", token: "", utente: "", password: "", oauth2: null },
      body: JSON.stringify({ query, variables }), body_mode: "raw",
      form: [], tests: [], pre_script: "", post_script: "",
      impostazioni: { timeout_ms: 0, segui_redirect: true, verifica_tls: true, retry_429: 0 }, tags: [],
    };
  }

  async function esegui() {
    inCorso = true; errore = null; risposta = null;
    try {
      let vars = {};
      try { vars = tab.variabili ? JSON.parse(tab.variabili) : {}; } catch { errore = "Variabili: JSON non valido"; inCorso = false; return; }
      risposta = await api.inviaRichiesta(reqBase(tab.query, vars), variabili || {}, null);
    } catch (e) { errore = String(e); } finally { inCorso = false; }
  }

  const INTROSPECT = `query{__schema{types{name kind fields{name type{name kind ofType{name kind ofType{name}}}}}}}`;
  function nomeTipo(t) {
    if (!t) return "?";
    return t.name || nomeTipo(t.ofType) || t.kind || "?";
  }
  async function introspeziona() {
    inCorso = true; errore = null;
    try {
      const resp = await api.inviaRichiesta(reqBase(INTROSPECT, {}), variabili || {}, null);
      const data = JSON.parse(resp.body);
      const tipi = data?.data?.__schema?.types || [];
      schema = tipi
        .filter((t) => t.kind === "OBJECT" && !t.name.startsWith("__") && t.fields)
        .map((t) => ({ name: t.name, fields: t.fields.map((f) => ({ name: f.name, type: nomeTipo(f.type) })) }));
    } catch (e) { errore = "Introspezione fallita: " + e; } finally { inCorso = false; }
  }
  const schemaFiltrato = $derived(
    schema ? schema.filter((t) => !cerca || t.name.toLowerCase().includes(cerca.toLowerCase()) || t.fields.some((f) => f.name.toLowerCase().includes(cerca.toLowerCase()))) : []
  );
</script>

<div class="gql">
  <div class="bar">
    <span class="proto">GraphQL</span>
    <input class="url" placeholder="https://…/graphql" bind:value={tab.url} />
    <button class="btn run" onclick={esegui} disabled={inCorso}>{inCorso ? "…" : "▶ Esegui"}</button>
    <button class="btn" onclick={introspeziona} disabled={inCorso}>Introspezione</button>
  </div>

  <div class="cols">
    <div class="left">
      <label>Query</label>
      <textarea class="q" bind:value={tab.query} spellcheck="false" placeholder={"query {\n  ...\n}"}></textarea>
      <label>Variabili (JSON)</label>
      <textarea class="v" bind:value={tab.variabili} spellcheck="false" placeholder={'{ "id": 1 }'}></textarea>
    </div>

    <div class="right">
      {#if schema}
        <div class="schema">
          <input class="sc" placeholder="🔍 cerca tipo/campo…" bind:value={cerca} />
          {#each schemaFiltrato as t}
            <div class="tp">{t.name}</div>
            {#each t.fields as f}
              <div class="fl"><span class="fn">{f.name}</span><span class="ft">{f.type}</span></div>
            {/each}
          {/each}
        </div>
      {:else if errore}
        <div class="err">{errore}</div>
      {:else if risposta}
        <div class="resp">
          <div class="st {risposta.status < 400 ? 'ok' : 'ko'}">{risposta.status} · {risposta.tempo_ms} ms</div>
          <pre>{(() => { try { return JSON.stringify(JSON.parse(risposta.body), null, 2); } catch { return risposta.body; } })()}</pre>
        </div>
      {:else}
        <div class="vuoto">Esegui una query o premi <b>Introspezione</b> per esplorare lo schema.</div>
      {/if}
    </div>
  </div>
</div>

<style>
  .gql { display: flex; flex-direction: column; height: 100%; min-height: 0; }
  .bar { display: flex; gap: 8px; align-items: center; padding: 12px 16px; border-bottom: 1px solid var(--border); }
  .proto { font-family: var(--mono); font-weight: 700; font-size: 11px; padding: 3px 8px; border-radius: 6px; background: var(--panel-3); color: #e535ab; }
  .url { flex: 1; background: var(--panel-2); border: 1px solid var(--border); border-radius: 7px; padding: 8px 10px; color: var(--txt); font-family: var(--mono); font-size: 13px; outline: none; }
  .url:focus { border-color: var(--accent); }
  .btn { border: 1px solid var(--border-2); background: var(--panel-3); color: var(--txt); border-radius: 7px; padding: 8px 12px; font-size: 13px; cursor: pointer; }
  .btn.run { background: linear-gradient(145deg,#8b6dff,#6c47ff); border: none; color: #fff; }
  .btn:disabled { opacity: .5; }
  .cols { flex: 1; display: grid; grid-template-columns: 1fr 1fr; min-height: 0; }
  .left { display: flex; flex-direction: column; padding: 12px 14px; border-right: 1px solid var(--border); min-height: 0; }
  .right { padding: 12px 14px; overflow: auto; min-height: 0; }
  label { font-size: 11px; text-transform: uppercase; letter-spacing: .05em; color: var(--txt-faint); margin: 8px 0 4px; }
  textarea { background: var(--panel-2); border: 1px solid var(--border); border-radius: 7px; padding: 8px 10px; color: var(--txt); font-family: var(--mono); font-size: 12.5px; outline: none; resize: none; }
  textarea.q { flex: 1; min-height: 120px; } textarea.v { height: 90px; }
  textarea:focus { border-color: var(--accent); }
  .sc { width: 100%; background: var(--panel-2); border: 1px solid var(--border); border-radius: 6px; padding: 6px 9px; color: var(--txt); font-size: 12px; outline: none; margin-bottom: 8px; }
  .tp { font-family: var(--mono); font-weight: 700; color: var(--accent-2); margin: 10px 0 3px; }
  .fl { display: flex; justify-content: space-between; font-family: var(--mono); font-size: 12px; padding: 2px 0 2px 12px; }
  .fn { color: var(--txt); } .ft { color: var(--txt-faint); }
  .resp pre { background: #0b0e14; border: 1px solid var(--border); border-radius: 7px; padding: 10px 12px; font-size: 12.5px; white-space: pre-wrap; word-break: break-word; }
  .st { font-family: var(--mono); font-size: 12px; margin-bottom: 6px; } .st.ok { color: var(--green); } .st.ko { color: var(--red); }
  .err { color: var(--red); font-size: 13px; } .vuoto { color: var(--txt-faint); font-size: 13px; }
</style>
