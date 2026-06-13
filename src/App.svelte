<script>
  // Componente principale: barra attività + vista nella sidebar + area centrale a tab.
  import { onMount } from "svelte";
  import * as api from "./lib/api.js";
  import { settings } from "./lib/settings.svelte.js";
  import { eseguiPre, eseguiPost, rispostaToRes } from "./lib/pm.js";
  import { eseguiCatena } from "./lib/runner.js";
  import { layout, ridimensiona } from "./lib/layout.svelte.js";
  import { logga } from "./lib/log.svelte.js";
  import Titlebar from "./components/Titlebar.svelte";
  import Rail from "./components/Rail.svelte";
  import CollectionsView from "./views/CollectionsView.svelte";
  import GitView from "./views/GitView.svelte";
  import WorkspacesView from "./views/WorkspacesView.svelte";
  import SettingsView from "./views/SettingsView.svelte";
  import EnvironmentsView from "./views/EnvironmentsView.svelte";
  import RunView from "./views/RunView.svelte";
  import HistoryView from "./views/HistoryView.svelte";
  import InfoView from "./views/InfoView.svelte";
  import Editor from "./components/Editor.svelte";
  import Response from "./components/Response.svelte";
  import Performance from "./components/Performance.svelte";
  import DiffView from "./components/DiffView.svelte";
  import RunResults from "./components/RunResults.svelte";
  import LogPanel from "./components/LogPanel.svelte";
  import Splitter from "./components/Splitter.svelte";

  let vista = $state("collezioni"); // collezioni | git | workspaces | settings

  let albero = $state([]);
  let environments = $state([]);
  let ambienteAttivo = $state(null);
  let storia = $state([]);

  // Nome dell'ambiente attivo (per la cronologia).
  const nomeAmbienteAttivo = $derived.by(() => {
    const env = environments.find((e) => e.file === ambienteAttivo);
    return env ? env.environment.nome : "";
  });

  // Tab aperti e id del tab attivo.
  let tabs = $state([]);
  let tabAttivoId = $state(null);
  let prossimoId = 1;

  let segnaleGit = $state(0);

  const tabAttivo = $derived(tabs.find((t) => t.id === tabAttivoId) ?? null);

  const variabiliAttive = $derived.by(() => {
    const env = environments.find((e) => e.file === ambienteAttivo);
    if (!env) return null;
    const m = {};
    for (const v of env.environment.variabili) if (v.chiave) m[v.chiave] = v.valore;
    return m;
  });

  // ---------------- Caricamento dati ----------------
  async function ricaricaAlbero() {
    try { albero = await api.caricaAlbero(); } catch (e) { console.error(e); }
  }
  async function ricaricaEnvironments() {
    try {
      environments = await api.caricaEnvironments();
      if (ambienteAttivo && !environments.some((e) => e.file === ambienteAttivo)) ambienteAttivo = null;
    } catch (e) { console.error(e); }
  }
  async function ricaricaStoria() {
    try { storia = await api.caricaStoria(); } catch (e) { console.error(e); }
  }
  async function pulisciStoria() {
    try { await api.pulisciStoria(); storia = []; } catch (e) { console.error(e); }
  }
  onMount(async () => {
    await Promise.all([ricaricaAlbero(), ricaricaEnvironments(), ricaricaStoria()]);
  });

  // ---------------- History / replay ----------------
  // Riapre una richiesta dalla cronologia in un tab "volante" (senza file),
  // pronta da reinviare con il pulsante Send.
  function apriDaStoria(voce) {
    const r = structuredClone($state.snapshot(voce.richiesta));
    r.headers ??= []; r.params ??= []; r.tests ??= [];
    r.auth ??= { tipo: "none", token: "", utente: "", password: "", oauth2: null };
    r.pre_script ??= ""; r.post_script ??= "";
    const tab = {
      id: prossimoId++, tipo: "request", file: null, dir: "",
      collezione: "(history)", richiesta: r,
      salvato: "", risposta: null, risultatiTest: [], inCorso: false, errore: null,
    };
    tabs.push(tab); tabAttivoId = tab.id;
  }

  // ---------------- Gestione tab ----------------
  function nomeCollezione(file) {
    const c = albero.find((c) => file === c.dir || file.startsWith(c.dir + "/"));
    return c ? c.nome : "";
  }

  function apriRichiesta(file, richiesta) {
    const esistente = tabs.find((t) => t.tipo === "request" && t.file === file);
    if (esistente) { tabAttivoId = esistente.id; return; }
    const r = structuredClone($state.snapshot(richiesta));
    r.headers ??= []; r.params ??= []; r.tests ??= [];
    r.auth ??= { tipo: "none", token: "", utente: "", password: "", oauth2: null };
    r.pre_script ??= ""; r.post_script ??= "";
    const dir = file.slice(0, file.lastIndexOf("/"));
    const tab = {
      id: prossimoId++, tipo: "request", file, dir,
      collezione: nomeCollezione(file), richiesta: r,
      salvato: JSON.stringify(r), risposta: null, risultatiTest: [],
      inCorso: false, errore: null,
    };
    tabs.push(tab);
    tabAttivoId = tab.id;
  }

  async function apriDiff(file) {
    let righe = [];
    try { righe = await api.gitDiff(file); } catch (e) { console.error(e); }
    const esistente = tabs.find((t) => t.tipo === "diff" && t.file === file);
    if (esistente) { esistente.righe = righe; tabAttivoId = esistente.id; return; }
    const tab = { id: prossimoId++, tipo: "diff", file, titolo: `Diff · ${file}`, righe };
    tabs.push(tab);
    tabAttivoId = tab.id;
  }

  function chiudiTab(id) {
    const i = tabs.findIndex((t) => t.id === id);
    if (i < 0) return;
    tabs.splice(i, 1);
    if (tabAttivoId === id) tabAttivoId = tabs.length ? tabs[Math.max(0, i - 1)].id : null;
  }

  function dirty(t) {
    return t.tipo === "request" && JSON.stringify($state.snapshot(t.richiesta)) !== t.salvato;
  }

  // ---------------- Azioni sul tab attivo ----------------
  async function salva() {
    const t = tabAttivo;
    if (!t || t.tipo !== "request" || !t.dir) return;
    try {
      t.file = await api.salvaRichiesta(t.dir, t.file, $state.snapshot(t.richiesta));
      t.salvato = JSON.stringify($state.snapshot(t.richiesta));
      await ricaricaAlbero();
      segnaleGit++;
      logga("info", `Salvato ${t.file}`);
    } catch (e) { t.errore = String(e); logga("errore", `Salvataggio fallito: ${e}`); }
  }

  async function invia() {
    const t = tabAttivo;
    if (!t || t.tipo !== "request") return;
    t.inCorso = true; t.errore = null; t.risultatiTest = [];
    try {
      // Le variabili partono dall'ambiente attivo; gli script possono modificarle.
      const vars = { ...(variabiliAttive || {}) };
      // Copia di lavoro: pre/post-script non alterano la richiesta salvata.
      const req = $state.snapshot(t.richiesta);
      const pre = eseguiPre(req.pre_script, { req, vars });
      t.risposta = await api.inviaRichiesta(req, vars);
      // Asserzioni dichiarative (backend) + post-script (pm.test).
      let tests = [];
      if (req.tests?.length) tests = await api.valutaTest(req.tests, t.risposta);
      const post = eseguiPost(req.post_script, { res: rispostaToRes(t.risposta), vars });
      t.risultatiTest = [...tests, ...post.tests];
      // Log: esito + eventuali messaggi degli script.
      const r = t.risposta;
      logga(r.status < 400 ? "ok" : "errore", `${req.metodo} ${req.url} → ${r.status} (${r.tempo_ms} ms)`);
      for (const l of [...pre.logs, ...post.logs]) logga("info", l);
      const ko = t.risultatiTest.filter((x) => !x.passato).length;
      if (ko) logga("errore", `${ko} test falliti`);
      // Registra nella cronologia (la richiesta com'è in editor, per il replay).
      try {
        await api.aggiungiStoria({
          quando: new Date().toISOString(),
          richiesta: $state.snapshot(t.richiesta),
          status: r.status, status_text: r.status_text,
          tempo_ms: r.tempo_ms, dimensione: r.dimensione,
          ambiente: nomeAmbienteAttivo,
        });
        if (vista === "storia") await ricaricaStoria();
      } catch (e) { console.error(e); }
    } catch (e) {
      t.errore = String(e);
      t.risposta = null;
      logga("errore", `Invio fallito: ${e}`);
    } finally { t.inCorso = false; }
  }

  // OAuth2: ottiene il token (risolvendo le variabili dell'ambiente attivo).
  async function ottieniTokenOauth(auth) {
    try {
      const token = await api.oauth2Token(auth, variabiliAttive || {});
      logga("ok", "Token OAuth2 ottenuto");
      return token;
    } catch (e) {
      logga("errore", `OAuth2: ${e}`);
      return null;
    }
  }
  // Copia negli appunti il comando cURL equivalente alla richiesta.
  async function copiaCurl(richiesta) {
    try {
      const cmd = await api.generaCurl($state.snapshot(richiesta));
      await navigator.clipboard.writeText(cmd);
      logga("ok", "Comando cURL copiato negli appunti");
    } catch (e) {
      logga("errore", `Copia cURL fallita: ${e}`);
    }
  }

  // Autosave con debounce: salva il tab attivo dopo `autosaveMs` di inattività.
  $effect(() => {
    if (!settings.autosave) return;
    const t = tabAttivo;
    if (!t || t.tipo !== "request" || !t.dir) return;
    const snap = JSON.stringify($state.snapshot(t.richiesta)); // dipendenza
    if (snap === t.salvato) return;
    const id = setTimeout(() => salva(), settings.autosaveMs);
    return () => clearTimeout(id);
  });

  // ---------------- Collezioni / cartelle ----------------
  async function nuovaCollezione(nome) { await api.creaCollezione(nome); await ricaricaAlbero(); segnaleGit++; }
  async function nuovaCartella(dirGenitore, nome) { await api.creaCartella(dirGenitore, nome); await ricaricaAlbero(); segnaleGit++; }
  async function nuovaRichiesta(dir, nome) {
    const rel = await api.creaRichiesta(dir, nome);
    await ricaricaAlbero();
    // trova la richiesta appena creata nell'albero e aprila
    const trova = (figli) => {
      for (const n of figli) {
        if (n.tipo === "richiesta" && n.file === rel) return n;
        if (n.tipo === "cartella") { const t = trova(n.figli); if (t) return t; }
      }
      return null;
    };
    for (const c of albero) { const n = trova(c.figli); if (n) { apriRichiesta(n.file, n.richiesta); break; } }
    segnaleGit++;
  }
  async function rinominaCartella(dir, nuovoNome) {
    const nuova = await api.rinominaCartella(dir, nuovoNome);
    // aggiorna i tab che puntavano dentro la cartella rinominata
    for (const t of tabs) {
      if (t.file && (t.file === dir || t.file.startsWith(dir + "/"))) {
        t.file = nuova + t.file.slice(dir.length);
        if (t.dir) t.dir = nuova + t.dir.slice(dir.length);
        if (t.collezione !== undefined) t.collezione = nomeCollezione(t.file);
      }
    }
    await ricaricaAlbero(); segnaleGit++;
  }
  async function eliminaCartella(dir) {
    await api.eliminaCartella(dir);
    tabs = tabs.filter((t) => !(t.file && (t.file === dir || t.file.startsWith(dir + "/"))));
    if (!tabs.some((t) => t.id === tabAttivoId)) tabAttivoId = tabs.length ? tabs[0].id : null;
    await ricaricaAlbero(); segnaleGit++;
  }
  async function eliminaRichiesta(file) {
    await api.elimina(file);
    const t = tabs.find((t) => t.file === file);
    if (t) chiudiTab(t.id);
    await ricaricaAlbero(); segnaleGit++;
  }

  // ---------------- Import / Export ----------------
  async function esportaCollezione(dir, nome) {
    const json = await api.esportaCollezione(dir);
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url; a.download = `${nome || dir}.rustman.json`;
    document.body.appendChild(a); a.click(); a.remove(); URL.revokeObjectURL(url);
  }
  async function importaCollezione(contenuto) {
    try {
      // Import "smart": riconosce Rustman e Postman (collection o environment).
      const esito = await api.importa(contenuto);
      if (esito.tipo === "environment") {
        await ricaricaEnvironments();
        logga("ok", `Ambiente importato (${esito.file})`);
      } else {
        await ricaricaAlbero();
        logga("ok", `Collezione importata in ${esito.dir}`);
        // Una collezione Postman con variabili crea anche un ambiente.
        if (esito.environment) {
          await ricaricaEnvironments();
          logga("ok", `Variabili importate come ambiente (${esito.environment})`);
        }
      }
      segnaleGit++;
    } catch (e) {
      logga("errore", `Import fallito: ${e}`);
    }
  }

  // ---------------- Ambienti ----------------
  async function salvaEnv(filePrecedente, environment) {
    const file = await api.salvaEnvironment(filePrecedente, environment);
    await ricaricaEnvironments(); segnaleGit++; return file;
  }
  async function eliminaEnv(file) { await api.eliminaEnvironment(file); await ricaricaEnvironments(); segnaleGit++; }

  // ---------------- Workspace ----------------
  async function cambiaWorkspace() {
    tabs = []; tabAttivoId = null; ambienteAttivo = null;
    await Promise.all([ricaricaAlbero(), ricaricaEnvironments()]);
    segnaleGit++;
    vista = "collezioni";
    logga("info", "Workspace cambiato");
  }

  // Esegue una catena e ne mostra i risultati in un tab dedicato.
  async function eseguiRun(catena) {
    const tab = { id: prossimoId++, tipo: "run", titolo: `Run · ${catena.nome}`, risultati: [], inCorso: true };
    tabs.push(tab); tabAttivoId = tab.id;
    logga("info", `Avvio catena "${catena.nome}" (${catena.passi.length} passi)`);
    try {
      tab.risultati = await eseguiCatena(catena, $state.snapshot(albero), variabiliAttive || {});
      const ok = tab.risultati.filter((r) => r.ok).length;
      logga(ok === tab.risultati.length ? "ok" : "errore",
        `Catena "${catena.nome}": ${ok}/${tab.risultati.length} passi ok`);
    } catch (e) {
      tab.risultati = [{ nome: "errore", ok: false, errore: String(e), tests: [], logs: [] }];
      logga("errore", `Catena fallita: ${e}`);
    } finally {
      tab.inCorso = false;
    }
  }

  function classeMetodo(m) {
    if (m === "GET") return "get"; if (m === "POST") return "post";
    if (m === "PUT" || m === "PATCH") return "put"; if (m === "DELETE") return "del";
    return "";
  }
</script>

<div class="app">
  <Titlebar />
  <div class="body" style="grid-template-columns: 48px {layout.sidebar}px 5px 1fr">
    <Rail {vista} onCambiaVista={(v) => (vista = v)} />

    <div class="sidebar">
      {#if vista === "collezioni"}
        <CollectionsView
          {albero}
          attivo={tabAttivo?.file ?? null}
          onApri={apriRichiesta}
          onNuovaCollezione={nuovaCollezione}
          onNuovaCartella={nuovaCartella}
          onNuovaRichiesta={nuovaRichiesta}
          onRinomina={rinominaCartella}
          onEliminaCartella={eliminaCartella}
          onEliminaRichiesta={eliminaRichiesta}
          onEsporta={esportaCollezione}
          onImporta={importaCollezione}
        />
      {:else if vista === "run"}
        <RunView {albero} onEsegui={eseguiRun} />
      {:else if vista === "storia"}
        <HistoryView
          {storia}
          onApri={apriDaStoria}
          onAggiorna={ricaricaStoria}
          onPulisci={pulisciStoria}
        />
      {:else if vista === "git"}
        <GitView segnale={segnaleGit} onApriDiff={apriDiff} onCambiamento={ricaricaAlbero} />
      {:else if vista === "ambienti"}
        <EnvironmentsView
          {environments}
          {ambienteAttivo}
          onSalva={salvaEnv}
          onElimina={eliminaEnv}
          onImpostaAttivo={(f) => (ambienteAttivo = f)}
        />
      {:else if vista === "workspaces"}
        <WorkspacesView onCambiato={cambiaWorkspace} />
      {:else if vista === "info"}
        <InfoView />
      {:else}
        <SettingsView />
      {/if}
    </div>

    <Splitter direction="col" onResize={(d) => ridimensiona("sidebar", d, 150, 520)} />

    <div class="center">
      <!-- Barra dei tab -->
      <div class="tabbar">
        {#each tabs as t (t.id)}
          <div class="tab" class:active={t.id === tabAttivoId} onclick={() => (tabAttivoId = t.id)}>
            {#if t.tipo === "request"}
              <span class="m {classeMetodo(t.richiesta.metodo)}">{t.richiesta.metodo}</span>
              {#if t.collezione}<span class="coll">{t.collezione} /</span>{/if}
              <span class="titolo">{t.richiesta.nome || "(senza nome)"}</span>
              {#if dirty(t)}<span class="punto"></span>{/if}
            {:else}
              <span class="titolo">{t.titolo}</span>
            {/if}
            <span class="chiudi" onclick={(e) => { e.stopPropagation(); chiudiTab(t.id); }}>✕</span>
          </div>
        {/each}
      </div>

      <!-- Contenuto del tab attivo -->
      <div class="center-main">
        {#if !tabAttivo}
          <div class="center-vuoto">Apri una richiesta dalle Collections.</div>
        {:else if tabAttivo.tipo === "diff"}
          <DiffView titolo={tabAttivo.titolo} righe={tabAttivo.righe} />
        {:else if tabAttivo.tipo === "run"}
          {#if tabAttivo.inCorso}
            <div class="center-vuoto">Esecuzione in corso…</div>
          {:else}
            <RunResults titolo={tabAttivo.titolo} risultati={tabAttivo.risultati} />
          {/if}
        {:else}
          <div class="req-area" style="grid-template-columns: 1fr 5px {layout.right}px">
            <div class="editor-col">
              <Editor
                richiesta={tabAttivo.richiesta}
                inCorso={tabAttivo.inCorso}
                salvabile={!!tabAttivo.dir}
                {environments}
                {ambienteAttivo}
                onCambiaAmbiente={(f) => (ambienteAttivo = f)}
                onApriEnv={() => (vista = "ambienti")}
                onInvia={invia}
                onSalva={salva}
                onCopiaCurl={copiaCurl}
                onOttieniToken={ottieniTokenOauth}
              />
            </div>
            <Splitter direction="col" onResize={(d) => ridimensiona("right", -d, 280, 900)} />
            <div class="right" style="grid-template-rows: 1fr 5px {layout.perf}px">
              <Response
                risposta={tabAttivo.risposta}
                inCorso={tabAttivo.inCorso}
                errore={tabAttivo.errore}
                risultatiTest={tabAttivo.risultatiTest}
              />
              <Splitter direction="row" onResize={(d) => ridimensiona("perf", -d, 120, 700)} />
              <Performance richiesta={tabAttivo.richiesta} variabili={variabiliAttive} />
            </div>
          </div>
        {/if}
      </div>

      <!-- Pannello Log ridimensionabile in basso -->
      <Splitter direction="row" onResize={(d) => ridimensiona("log", -d, 0, 500)} />
      <div class="logwrap" style="height: {layout.log}px">
        <LogPanel />
      </div>
    </div>
  </div>
</div>
