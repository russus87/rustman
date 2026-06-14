<script>
  // Pannello "Strumenti": utilità per sviluppatori API (tutto lato webview).
  import * as api from "../lib/api.js";
  let { environments = [], onImportaRichiesta } = $props();

  let sez = $state("jwt"); // jwt | base64 | url | ts | hmac | import | envdiff
  const sezioni = [
    ["jwt", "JWT"], ["base64", "Base64"], ["url", "URL"], ["ts", "Timestamp"],
    ["hmac", "HMAC"], ["import", "Importa"], ["envdiff", "Diff ambienti"],
  ];

  // ---- JWT ----
  let jwt = $state("");
  function b64url(s) {
    try {
      const p = s.replace(/-/g, "+").replace(/_/g, "/");
      return decodeURIComponent(atob(p).split("").map((c) => "%" + c.charCodeAt(0).toString(16).padStart(2, "0")).join(""));
    } catch { return null; }
  }
  const jwtDecoded = $derived.by(() => {
    const parti = jwt.trim().split(".");
    if (parti.length < 2) return null;
    try {
      const header = JSON.parse(b64url(parti[0]));
      const payload = JSON.parse(b64url(parti[1]));
      let scadenza = null;
      if (payload.exp) scadenza = { data: new Date(payload.exp * 1000).toLocaleString(), scaduto: payload.exp * 1000 < Date.now() };
      return { header, payload, scadenza };
    } catch { return null; }
  });

  // ---- Base64 ----
  let b64in = $state("");
  function b64enc(s) { try { return btoa(unescape(encodeURIComponent(s))); } catch { return "(errore)"; } }
  function b64dec(s) { try { return decodeURIComponent(escape(atob(s))); } catch { return "(non valido)"; } }

  // ---- URL ----
  let urlin = $state("");

  // ---- Timestamp ----
  let tsin = $state(String(Math.floor(Date.now() / 1000)));
  const tsConv = $derived.by(() => {
    const n = Number(tsin);
    if (!n) return null;
    const ms = String(tsin).length > 10 ? n : n * 1000;
    try { return new Date(ms).toISOString(); } catch { return null; }
  });

  // ---- HMAC ----
  let hmacKey = $state("");
  let hmacMsg = $state("");
  let hmacAlgo = $state("SHA-256");
  let hmacOut = $state("");
  async function calcolaHmac() {
    try {
      const enc = new TextEncoder();
      const k = await crypto.subtle.importKey("raw", enc.encode(hmacKey), { name: "HMAC", hash: hmacAlgo }, false, ["sign"]);
      const sig = await crypto.subtle.sign("HMAC", k, enc.encode(hmacMsg));
      hmacOut = [...new Uint8Array(sig)].map((b) => b.toString(16).padStart(2, "0")).join("");
    } catch (e) { hmacOut = "(errore: " + e + ")"; }
  }

  // ---- Importa cURL / fetch ----
  let impTesto = $state("");
  let impErr = $state("");
  function parseFetch(t) {
    const m = t.match(/fetch\(\s*["'`]([^"'`]+)["'`]/);
    if (!m) return null;
    const url = m[1];
    let opts = {};
    const i = t.indexOf("{", t.indexOf(m[0]) + m[0].length);
    if (i >= 0) {
      let depth = 0, j = i;
      for (; j < t.length; j++) { if (t[j] === "{") depth++; else if (t[j] === "}") { depth--; if (depth === 0) { j++; break; } } }
      try { opts = JSON.parse(t.slice(i, j)); } catch { /* opzioni non JSON */ }
    }
    const headers = Object.entries(opts.headers || {}).map(([k, v]) => ({ chiave: k, valore: String(v), attivo: true }));
    return { nome: "Importata da fetch", metodo: (opts.method || "GET").toUpperCase(), url, headers,
      params: [], auth: { tipo: "none", token: "", utente: "", password: "", oauth2: null },
      body: typeof opts.body === "string" ? opts.body : "", body_mode: "raw", form: [], tests: [], pre_script: "", post_script: "" };
  }
  async function importa() {
    impErr = "";
    const t = impTesto.trim();
    try {
      let r;
      if (t.startsWith("fetch")) r = parseFetch(t);
      else r = await api.importaCurl(t);
      if (!r) { impErr = "Non riconosciuto (incolla un comando cURL o una chiamata fetch)."; return; }
      onImportaRichiesta?.(r);
      impTesto = "";
    } catch (e) { impErr = String(e); }
  }

  // ---- Diff ambienti ----
  let envA = $state("");
  let envB = $state("");
  const envDiff = $derived.by(() => {
    const a = environments.find((e) => e.file === envA)?.environment;
    const b = environments.find((e) => e.file === envB)?.environment;
    if (!a || !b) return [];
    const mapA = Object.fromEntries(a.variabili.map((v) => [v.chiave, v.valore]));
    const mapB = Object.fromEntries(b.variabili.map((v) => [v.chiave, v.valore]));
    const chiavi = [...new Set([...Object.keys(mapA), ...Object.keys(mapB)])].sort();
    return chiavi.map((k) => ({ k, a: mapA[k], b: mapB[k], stato: mapA[k] === mapB[k] ? "uguale" : (mapA[k] === undefined || mapB[k] === undefined ? "solo" : "diverso") }));
  });
</script>

<div class="str">
  <div class="tabs">
    {#each sezioni as [k, lbl]}
      <div class="t" class:active={sez === k} onclick={() => (sez = k)}>{lbl}</div>
    {/each}
  </div>

  <div class="body">
    {#if sez === "jwt"}
      <label>Token JWT</label>
      <textarea bind:value={jwt} placeholder="eyJhbGciOi..."></textarea>
      {#if jwtDecoded}
        {#if jwtDecoded.scadenza}
          <div class="info {jwtDecoded.scadenza.scaduto ? 'ko' : 'ok'}">Scadenza: {jwtDecoded.scadenza.data} {jwtDecoded.scadenza.scaduto ? "(SCADUTO)" : ""}</div>
        {/if}
        <label>Header</label><pre>{JSON.stringify(jwtDecoded.header, null, 2)}</pre>
        <label>Payload</label><pre>{JSON.stringify(jwtDecoded.payload, null, 2)}</pre>
      {:else if jwt.trim()}<div class="info ko">Token non valido.</div>{/if}

    {:else if sez === "base64"}
      <label>Testo</label><textarea bind:value={b64in}></textarea>
      <label>Encode</label><pre>{b64in ? b64enc(b64in) : ""}</pre>
      <label>Decode</label><pre>{b64in ? b64dec(b64in) : ""}</pre>

    {:else if sez === "url"}
      <label>Testo</label><textarea bind:value={urlin}></textarea>
      <label>Encode</label><pre>{urlin ? encodeURIComponent(urlin) : ""}</pre>
      <label>Decode</label><pre>{urlin ? (() => { try { return decodeURIComponent(urlin); } catch { return "(non valido)"; } })() : ""}</pre>

    {:else if sez === "ts"}
      <label>Unix timestamp (s o ms)</label><input bind:value={tsin} />
      <label>ISO 8601 (UTC)</label><pre>{tsConv ?? "(non valido)"}</pre>
      <div class="info">Ora: {Math.floor(Date.now() / 1000)} ({new Date().toLocaleString()})</div>

    {:else if sez === "hmac"}
      <label>Algoritmo</label>
      <select bind:value={hmacAlgo}><option>SHA-256</option><option>SHA-1</option><option>SHA-384</option><option>SHA-512</option></select>
      <label>Chiave</label><input bind:value={hmacKey} />
      <label>Messaggio</label><textarea bind:value={hmacMsg}></textarea>
      <button class="btn" onclick={calcolaHmac}>Calcola HMAC</button>
      {#if hmacOut}<label>Firma (hex)</label><pre>{hmacOut}</pre>{/if}

    {:else if sez === "import"}
      <label>Incolla un comando cURL o una chiamata fetch()</label>
      <textarea bind:value={impTesto} placeholder={"curl -X POST ...   oppure   fetch(\"...\", { ... })"}></textarea>
      <button class="btn" onclick={importa}>Importa come richiesta</button>
      {#if impErr}<div class="info ko">{impErr}</div>{/if}

    {:else if sez === "envdiff"}
      <div class="row">
        <select bind:value={envA}><option value="">Ambiente A…</option>{#each environments as e}<option value={e.file}>{e.environment.nome}</option>{/each}</select>
        <select bind:value={envB}><option value="">Ambiente B…</option>{#each environments as e}<option value={e.file}>{e.environment.nome}</option>{/each}</select>
      </div>
      {#if envDiff.length}
        <table class="dt"><thead><tr><th>Variabile</th><th>A</th><th>B</th></tr></thead><tbody>
          {#each envDiff as d}
            <tr class={d.stato}><td class="k">{d.k}</td><td>{d.a ?? "—"}</td><td>{d.b ?? "—"}</td></tr>
          {/each}
        </tbody></table>
      {/if}
    {/if}
  </div>
</div>

<style>
  .str { display: flex; flex-direction: column; height: 100%; min-height: 0; }
  .tabs { display: flex; gap: 2px; padding: 8px 12px; border-bottom: 1px solid var(--border); flex-wrap: wrap; }
  .t { padding: 6px 12px; border-radius: 6px; cursor: pointer; color: var(--txt-dim); font-size: 12.5px; }
  .t:hover { background: var(--panel-3); } .t.active { background: rgba(124,92,255,.18); color: var(--txt); }
  .body { padding: 16px 20px; overflow-y: auto; max-width: 760px; }
  label { display: block; font-size: 11px; text-transform: uppercase; letter-spacing: .05em; color: var(--txt-faint); margin: 12px 0 4px; }
  textarea, input, select { width: 100%; background: var(--panel-2); border: 1px solid var(--border); border-radius: 7px; padding: 8px 10px; color: var(--txt); font-family: var(--mono); font-size: 12.5px; outline: none; }
  textarea { min-height: 70px; resize: vertical; }
  textarea:focus, input:focus, select:focus { border-color: var(--accent); }
  pre { background: #0b0e14; border: 1px solid var(--border); border-radius: 7px; padding: 10px 12px; overflow: auto; font-size: 12.5px; white-space: pre-wrap; word-break: break-all; color: var(--txt); }
  .btn { margin-top: 10px; background: linear-gradient(145deg,#8b6dff,#6c47ff); border: none; color: #fff; border-radius: 7px; padding: 8px 16px; font-size: 13px; cursor: pointer; }
  .info { margin: 8px 0; font-size: 12.5px; color: var(--txt-dim); }
  .info.ok { color: var(--green); } .info.ko { color: var(--red); }
  .row { display: flex; gap: 10px; margin-bottom: 12px; }
  .dt { width: 100%; border-collapse: collapse; font-size: 12.5px; font-family: var(--mono); }
  .dt th { text-align: left; color: var(--txt-faint); font-size: 11px; padding: 6px 10px; border-bottom: 1px solid var(--border); }
  .dt td { padding: 5px 10px; border-bottom: 1px solid var(--border); }
  .dt td.k { color: var(--accent-2); }
  .dt tr.diverso td { background: rgba(226,179,64,.08); }
  .dt tr.solo td { background: rgba(248,81,73,.08); }
</style>
