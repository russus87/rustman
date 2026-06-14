<script>
  // Componente principale: barra attività + vista nella sidebar + area centrale a tab.
  import { onMount } from "svelte";
  import * as api from "./lib/api.js";
  import { settings, applicaTema } from "./lib/settings.svelte.js";
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
  import CommandPalette from "./components/CommandPalette.svelte";
  import FolderConfig from "./components/FolderConfig.svelte";
  import Socket from "./components/Socket.svelte";
  import BatchResults from "./components/BatchResults.svelte";
  import Strumenti from "./components/Strumenti.svelte";
  import CheatSheet from "./components/CheatSheet.svelte";
  import GraphQL from "./components/GraphQL.svelte";
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
  let runs = $state([]);
  let percorsoWs = $state(""); // path del workspace (per preferiti/filtri salvati)
  async function ricaricaPercorso() {
    try { percorsoWs = await api.percorsoWorkspace(); } catch { percorsoWs = ""; }
  }

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
  async function ricaricaRuns() {
    try { runs = await api.caricaRuns(); } catch (e) { console.error(e); }
  }
  async function pulisciStoria() {
    try { await api.pulisciStoria(); storia = []; } catch (e) { console.error(e); }
  }
  onMount(async () => {
    applicaTema();
    await Promise.all([ricaricaAlbero(), ricaricaEnvironments(), ricaricaStoria(), ricaricaRuns(), ricaricaPercorso()]);
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

  // Cattura un valore dalla risposta in una variabile dell'ambiente attivo.
  async function capturaVar(path, value) {
    const env = environments.find((e) => e.file === ambienteAttivo);
    if (!env) { logga("errore", "Attiva un ambiente per salvare la variabile"); return; }
    const chiave = path.split(".").pop() || "valore";
    const vars = env.environment.variabili.filter((v) => v.chiave !== chiave);
    vars.push({ chiave, valore: value, segreto: false });
    const nuovo = { ...env.environment, variabili: vars };
    try {
      await api.salvaEnvironment(env.file, nuovo);
      await ricaricaEnvironments();
      logga("ok", `Variabile '${chiave}' = ${value} salvata in ${env.environment.nome}`);
    } catch (e) { logga("errore", `Salvataggio variabile fallito: ${e}`); }
  }
  // Genera asserzioni di base dalla risposta (status + campi JSON di primo livello).
  function autoTest(risposta) {
    const t = tabAttivo;
    if (!t || t.tipo !== "request") return;
    if (!t.richiesta.tests) t.richiesta.tests = [];
    t.richiesta.tests.push({ tipo: "status", operatore: "==", campo: "", atteso: String(risposta.status), attivo: true });
    let n = 1;
    try {
      const obj = JSON.parse(risposta.body);
      if (obj && typeof obj === "object" && !Array.isArray(obj)) {
        for (const [k, v] of Object.entries(obj).slice(0, 6)) {
          if (v === null || typeof v !== "object") {
            t.richiesta.tests.push({ tipo: "json", operatore: "==", campo: k, atteso: String(v), attivo: true });
            n++;
          }
        }
      }
    } catch { /* body non JSON */ }
    logga("ok", `${n} asserzioni generate dalla risposta`);
  }
  // Crea un'asserzione "schema" inferendo lo schema dalla risposta.
  async function autoSchema(risposta) {
    const t = tabAttivo;
    if (!t || t.tipo !== "request") return;
    try {
      const schema = await api.inferisciSchema(risposta.body);
      if (!t.richiesta.tests) t.richiesta.tests = [];
      t.richiesta.tests.push({ tipo: "schema", operatore: "", campo: "", atteso: schema, attivo: true });
      logga("ok", "Asserzione schema generata dalla risposta");
    } catch (e) { logga("errore", `Inferisci schema fallito: ${e}`); }
  }
  // Snapshot inline: diff tra baseline e risposta attuale.
  async function snapshotDiff() {
    const t = tabAttivo;
    if (!t || !t.file || !t.risposta) return;
    try {
      const base = (await api.caricaSnapshot(t.file)) ?? "";
      const righe = await api.diffTesti(base, t.risposta.body);
      const tab = { id: prossimoId++, tipo: "diff", file: null, titolo: `Snapshot · ${t.richiesta.nome || "richiesta"}`, righe };
      tabs.push(tab); tabAttivoId = tab.id;
    } catch (e) { logga("errore", `Diff snapshot fallito: ${e}`); }
  }
  async function snapshotAccetta() {
    const t = tabAttivo;
    if (!t || !t.file || !t.risposta) return;
    try {
      await api.aggiornaSnapshot(t.file, t.risposta.body);
      segnaleGit++;
      logga("ok", "Snapshot aggiornato");
    } catch (e) { logga("errore", `Aggiorna snapshot fallito: ${e}`); }
  }
  // Crea un'asserzione json sul tab attivo dal campo catturato.
  function creaTest(path, value) {
    const t = tabAttivo;
    if (!t || t.tipo !== "request") return;
    if (!t.richiesta.tests) t.richiesta.tests = [];
    t.richiesta.tests.push({ tipo: "json", operatore: "==", campo: path, atteso: value, attivo: true });
    logga("ok", `Asserzione aggiunta: json '${path}' == ${value}`);
  }

  // Diff delle modifiche non salvate del tab attivo (editor vs file salvato).
  async function diffNonSalvate() {
    const t = tabAttivo;
    if (!t || t.tipo !== "request") return;
    const attuale = JSON.stringify($state.snapshot(t.richiesta), null, 2);
    let salvato = attuale;
    try { salvato = JSON.stringify(JSON.parse(t.salvato), null, 2); } catch { salvato = ""; }
    let righe = [];
    try { righe = await api.diffTesti(salvato, attuale); } catch (e) { console.error(e); }
    const tab = { id: prossimoId++, tipo: "diff", file: null, titolo: `Modifiche · ${t.richiesta.nome || "richiesta"}`, righe };
    tabs.push(tab); tabAttivoId = tab.id;
  }

  // Confronta le risposte di due voci di cronologia in un tab diff.
  async function confrontaStoria(a, b) {
    let righe = [];
    try { righe = await api.diffTesti(a.body ?? "", b.body ?? ""); } catch (e) { console.error(e); }
    const tab = {
      id: prossimoId++, tipo: "diff", file: null,
      titolo: `Diff risposte · ${a.status} ↔ ${b.status}`, righe,
    };
    tabs.push(tab); tabAttivoId = tab.id;
  }
  // Esegue tutte le richieste sotto una cartella/collezione e mostra una griglia.
  async function eseguiBatch(dir) {
    const tab = { id: prossimoId++, tipo: "batch", dirBatch: dir, titolo: `Batch · ${dir}`, righe: [], inCorso: true, ognis: 0, _timer: null };
    tabs.push(tab); tabAttivoId = tab.id;
    await popolaBatch(tab);
  }
  async function popolaBatch(tab) {
    tab.inCorso = true; tab.righe = [];
    const tutte = [];
    for (const c of albero) for (const n of appiattisci(c.figli, [])) tutte.push(n);
    const sel = tutte.filter((n) => n.file.startsWith(tab.dirBatch + "/"));
    for (const n of sel) {
      const req = structuredClone($state.snapshot(n.richiesta));
      const riga = { nome: req.nome || "(senza nome)", metodo: req.metodo, url: req.url, status: 0, tempo: 0, ok: 0, tot: 0, errore: null };
      try {
        const dirR = n.file.slice(0, n.file.lastIndexOf("/"));
        const resp = await api.inviaRichiesta(req, variabiliAttive || {}, dirR);
        riga.status = resp.status; riga.tempo = resp.tempo_ms;
        if (req.tests?.length) {
          const r = await api.valutaTest(req.tests, resp);
          riga.tot = r.length; riga.ok = r.filter((x) => x.passato).length;
        }
      } catch (e) { riga.errore = String(e); }
      tab.righe.push(riga);
    }
    tab.inCorso = false;
  }
  // Scheduler: ri-esegue il batch a intervalli nello stesso tab.
  function pianificaBatch(tab, secondi) {
    fermaBatch(tab);
    if (secondi > 0) {
      tab.ognis = secondi;
      tab._timer = setInterval(() => popolaBatch(tab), secondi * 1000);
      logga("ok", `Scheduler avviato: "${tab.dirBatch}" ogni ${secondi}s`);
    }
  }
  function fermaBatch(tab) {
    if (tab._timer) { clearInterval(tab._timer); tab._timer = null; }
    tab.ognis = 0;
  }

  // Confronta le risposte di due tab aperti in un tab diff.
  async function confrontaRisposte(a, b, etichetta) {
    let righe = [];
    try { righe = await api.diffTesti(a ?? "", b ?? ""); } catch (e) { console.error(e); }
    const tab = { id: prossimoId++, tipo: "diff", file: null, titolo: `Diff · ${etichetta}`, righe };
    tabs.push(tab); tabAttivoId = tab.id;
  }

  // Apre una nuova console GraphQL in un tab.
  function nuovaGraphQL() {
    const tab = { id: prossimoId++, tipo: "graphql", titolo: "GraphQL", url: "", query: "query {\n  \n}", variabili: "{}" };
    tabs.push(tab); tabAttivoId = tab.id;
  }

  // Apre il pannello Strumenti in un tab.
  function apriStrumenti() {
    const esistente = tabs.find((t) => t.tipo === "strumenti");
    if (esistente) { tabAttivoId = esistente.id; return; }
    const tab = { id: prossimoId++, tipo: "strumenti", titolo: "🧰 Strumenti" };
    tabs.push(tab); tabAttivoId = tab.id;
  }
  // Apre una richiesta importata (da fetch/cURL) come tab "volante".
  function apriRichiestaImportata(r) {
    r.headers ??= []; r.params ??= []; r.tests ??= [];
    r.auth ??= { tipo: "none", token: "", utente: "", password: "", oauth2: null };
    r.pre_script ??= ""; r.post_script ??= "";
    const tab = { id: prossimoId++, tipo: "request", file: null, dir: "", collezione: "(import)",
      richiesta: r, salvato: "", risposta: null, risultatiTest: [], inCorso: false, errore: null };
    tabs.push(tab); tabAttivoId = tab.id;
    logga("ok", "Richiesta importata");
  }

  // Apre una nuova console WebSocket o SSE in un tab.
  function nuovaConnessione(protocollo) {
    const tab = {
      id: prossimoId++, tipo: "socket", protocollo, url: "",
      titolo: protocollo === "sse" ? "SSE" : "WebSocket",
    };
    tabs.push(tab); tabAttivoId = tab.id;
  }

  // Aggiorna (approva) la baseline dello snapshot della richiesta attiva.
  async function aggiornaSnapshotAttivo() {
    const t = tabAttivo;
    if (!t || t.tipo !== "request" || !t.file || !t.risposta) {
      logga("errore", "Invia prima una richiesta salvata per aggiornarne lo snapshot");
      return;
    }
    try {
      await api.aggiornaSnapshot(t.file, t.risposta.body);
      segnaleGit++;
      logga("ok", "Snapshot aggiornato (baseline)");
    } catch (e) { logga("errore", `Aggiorna snapshot fallito: ${e}`); }
  }
  // Copertura delle API: incrocia uno spec OpenAPI con le richieste con asserzioni.
  async function coverageDaSpec(spec, nome) {
    try {
      const c = await api.coverageOpenapi(spec);
      logga(c.scoperti.length === 0 ? "ok" : "info",
        `Coverage (${nome}): ${c.coperti}/${c.totali} (${c.percentuale.toFixed(0)}%)`);
      for (const s of c.scoperti) logga("errore", `scoperto: ${s}`);
    } catch (e) { logga("errore", `Coverage fallita: ${e}`); }
  }

  // Apre l'editor di configurazione ereditabile di una cartella in un tab.
  async function apriConfigCartella(dir, nome) {
    const esistente = tabs.find((t) => t.tipo === "cartella" && t.dir === dir);
    if (esistente) { tabAttivoId = esistente.id; return; }
    let config = { headers: [], auth: { tipo: "none", token: "", utente: "", password: "", oauth2: null } };
    try { config = await api.caricaConfigCartella(dir); } catch (e) { console.error(e); }
    const tab = { id: prossimoId++, tipo: "cartella", dir, nome, config, titolo: `⚙ ${nome}` };
    tabs.push(tab); tabAttivoId = tab.id;
  }
  async function salvaConfigCartella(dir, config) {
    try {
      await api.salvaConfigCartella(dir, config);
      segnaleGit++;
      logga("ok", `Configurazione della cartella salvata (${dir})`);
    } catch (e) { logga("errore", `Salvataggio config fallito: ${e}`); }
  }

  // Find & Replace su tutte le richieste delle collezioni.
  async function trovaSostituisci(cerca, con) {
    try {
      const n = await api.trovaSostituisci(cerca, con);
      await ricaricaAlbero(); segnaleGit++;
      logga("ok", `Sostituzione applicata a ${n} richieste`);
    } catch (e) { logga("errore", `Find&Replace fallito: ${e}`); }
  }
  // Drift detection fra due spec OpenAPI: mostra il report nel pannello Log.
  async function confrontaDrift(vecchio, nuovo, nomeA, nomeB) {
    try {
      const d = await api.driftOpenapi(vecchio, nuovo);
      logga("info", `Drift OpenAPI (${nomeA} → ${nomeB}): +${d.aggiunti.length} / -${d.rimossi.length} / ~${d.modificati.length}`);
      for (const x of d.aggiunti) logga("ok", `+ aggiunto: ${x}`);
      for (const x of d.rimossi) logga("errore", `- rimosso: ${x}`);
      for (const x of d.modificati) logga("info", `~ modificato: ${x}`);
      if (!d.aggiunti.length && !d.rimossi.length && !d.modificati.length) logga("ok", "Nessuna differenza tra gli spec.");
    } catch (e) { logga("errore", `Drift fallito: ${e}`); }
  }

  // Esporta l'intero workspace (collezioni + ambienti) come bundle.
  async function esportaWs() {
    try {
      const bundle = await api.esportaWorkspace();
      const blob = new Blob([bundle], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url; a.download = "rustman-workspace.json";
      document.body.appendChild(a); a.click(); a.remove(); URL.revokeObjectURL(url);
      logga("ok", "Workspace esportato");
    } catch (e) { logga("errore", `Export workspace fallito: ${e}`); }
  }
  async function importaWs(contenuto) {
    try {
      await api.importaWorkspace(contenuto);
      await Promise.all([ricaricaAlbero(), ricaricaEnvironments()]);
      segnaleGit++;
      logga("ok", "Workspace importato");
    } catch (e) { logga("errore", `Import workspace fallito: ${e}`); }
  }

  // Esporta le collezioni in uno spec OpenAPI e lo scarica.
  async function esportaOpenapi() {
    try {
      const spec = await api.esportaOpenapi();
      const blob = new Blob([spec], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url; a.download = "rustman-openapi.json";
      document.body.appendChild(a); a.click(); a.remove(); URL.revokeObjectURL(url);
      logga("ok", "Spec OpenAPI esportato");
    } catch (e) { logga("errore", `Export OpenAPI fallito: ${e}`); }
  }

  // Lint di uno spec OpenAPI: mostra i problemi nel pannello Log.
  async function lintDaSpec(spec, nome) {
    try {
      const issues = await api.lintOpenapi(spec);
      if (!issues.length) { logga("ok", `Lint (${nome}): nessun problema`); return; }
      logga("info", `Lint (${nome}): ${issues.length} rilievi`);
      for (const i of issues) logga(i.livello === "errore" ? "errore" : "info", `${i.livello}: ${i.messaggio}`);
    } catch (e) { logga("errore", `Lint fallito: ${e}`); }
  }

  // Diff fra due collezioni esportate (.rustman.json): mostra il report nel Log.
  async function diffCollezioni(a, b, nomeA, nomeB) {
    try {
      const d = await api.diffCollezioni(a, b);
      logga("info", `Diff collezioni (${nomeA} → ${nomeB}): +${d.aggiunti.length} / -${d.rimossi.length} / ~${d.modificati.length}`);
      for (const x of d.aggiunti) logga("ok", `+ aggiunta: ${x}`);
      for (const x of d.rimossi) logga("errore", `- rimossa: ${x}`);
      for (const x of d.modificati) logga("info", `~ modificata: ${x}`);
      if (!d.aggiunti.length && !d.rimossi.length && !d.modificati.length) logga("ok", "Collezioni identiche.");
    } catch (e) { logga("errore", `Diff collezioni fallito: ${e}`); }
  }

  // Genera la documentazione HTML e la scarica come file.
  async function esportaDoc() {
    try {
      const html = await api.generaDoc();
      const blob = new Blob([html], { type: "text/html" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url; a.download = "rustman-doc.html";
      document.body.appendChild(a); a.click(); a.remove(); URL.revokeObjectURL(url);
      logga("ok", "Documentazione HTML generata");
    } catch (e) {
      logga("errore", `Generazione doc fallita: ${e}`);
    }
  }

  function chiudiTab(id) {
    const i = tabs.findIndex((t) => t.id === id);
    if (i < 0) return;
    if (tabs[i]._timer) clearInterval(tabs[i]._timer); // ferma lo scheduler del batch
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
      // Variabili: collezione/cartella (priorità minore) < ambiente attivo.
      let vars = { ...(variabiliAttive || {}) };
      if (t.dir) {
        try {
          const cv = await api.variabiliCartella(t.dir);
          const m = {}; for (const v of cv) if (v.chiave) m[v.chiave] = v.valore;
          vars = { ...m, ...vars };
        } catch (e) { console.error(e); }
      }
      // Copia di lavoro: pre/post-script non alterano la richiesta salvata.
      const req = $state.snapshot(t.richiesta);
      const pre = eseguiPre(req.pre_script, { req, vars });
      t.risposta = await api.inviaRichiesta(req, vars, t.dir || null);
      // Asserzioni dichiarative (backend) + post-script (pm.test).
      let tests = [];
      if (req.tests?.length) tests = await api.valutaTest(req.tests, t.risposta);
      // Snapshot/golden testing: confronto con baseline (solo richieste salvate).
      if (t.file) {
        for (const a of req.tests || []) {
          if (a.attivo && a.tipo === "snapshot") {
            const ignora = (a.atteso || "").split(",").map((s) => s.trim()).filter(Boolean);
            try { tests.push(await api.valutaSnapshot(t.file, ignora, t.risposta)); } catch (e) { console.error(e); }
          }
        }
      }
      const post = eseguiPost(req.post_script, { res: rispostaToRes(t.risposta), vars });
      t.risultatiTest = [...tests, ...post.tests];
      // Security scan passivo della risposta.
      try { t.avvisiSicurezza = await api.securityScan(t.risposta); } catch { t.avvisiSicurezza = []; }
      // Log: esito + eventuali messaggi degli script.
      const r = t.risposta;
      logga(r.status < 400 ? "ok" : "errore", `${req.metodo} ${req.url} → ${r.status} (${r.tempo_ms} ms)`);
      for (const l of [...pre.logs, ...post.logs]) logga("info", l);
      const ko = t.risultatiTest.filter((x) => !x.passato).length;
      if (ko) logga("errore", `${ko} test falliti`);
      // Trend dei test: registra il riassunto dell'esecuzione.
      if (t.risultatiTest.length) {
        const okN = t.risultatiTest.length - ko;
        try {
          await api.registraRun({ quando: new Date().toISOString(), totali: t.risultatiTest.length,
            ok: okN, ko, etichetta: req.nome || req.url });
          if (vista === "storia") await ricaricaRuns();
        } catch (e) { console.error(e); }
      }
      // Registra nella cronologia (la richiesta com'è in editor, per il replay).
      try {
        await api.aggiungiStoria({
          quando: new Date().toISOString(),
          richiesta: $state.snapshot(t.richiesta),
          status: r.status, status_text: r.status_text,
          tempo_ms: r.tempo_ms, dimensione: r.dimensione,
          body: (r.body || "").slice(0, 200000), // troncato per il diff
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
  // Copia negli appunti lo snippet della richiesta nel linguaggio scelto.
  async function copiaCodice(richiesta, linguaggio = "curl") {
    try {
      const code = await api.generaCodice($state.snapshot(richiesta), linguaggio);
      await navigator.clipboard.writeText(code);
      logga("ok", `Copiato come ${linguaggio}`);
    } catch (e) {
      logga("errore", `Copia fallita: ${e}`);
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
    await Promise.all([ricaricaAlbero(), ricaricaEnvironments(), ricaricaPercorso()]);
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

  // ---------------- Command Palette (Ctrl/Cmd+K) ----------------
  let paletteAperta = $state(false);
  let cheatAperto = $state(false);

  // Tutte le richieste dell'albero, appiattite (file, nome, collezione).
  function appiattisci(figli, acc) {
    for (const n of figli) {
      if (n.tipo === "cartella") appiattisci(n.figli, acc);
      else acc.push(n);
    }
    return acc;
  }
  const comandi = $derived.by(() => {
    const out = [];
    // Richieste
    for (const c of albero) {
      for (const n of appiattisci(c.figli, [])) {
        out.push({
          tag: n.richiesta.metodo, tagClasse: classeMetodo(n.richiesta.metodo),
          label: n.richiesta.nome || "(senza nome)", hint: c.nome,
          azione: () => apriRichiesta(n.file, n.richiesta),
        });
      }
    }
    // Ambienti
    for (const e of environments) {
      out.push({ tag: "ENV", label: e.environment.nome, hint: "Attiva ambiente",
        azione: () => (ambienteAttivo = e.file) });
    }
    // Viste
    const viste = [["collezioni","Collections"],["run","Run"],["storia","History"],
      ["git","Git"],["ambienti","Environments"],["workspaces","Workspaces"],["info","Info"]];
    for (const [v, etichetta] of viste) {
      out.push({ tag: "VAI", label: etichetta, hint: "Apri vista", azione: () => (vista = v) });
    }
    // Azioni
    out.push({ tag: "⚡", label: "Invia richiesta", hint: "Ctrl+Invio", azione: invia });
    out.push({ tag: "⚡", label: "Genera documentazione", azione: esportaDoc });
    out.push({ tag: "⚡", label: "Aggiorna snapshot (richiesta attiva)", azione: aggiornaSnapshotAttivo });
    out.push({ tag: "🔌", label: "Nuova connessione WebSocket", azione: () => nuovaConnessione("ws") });
    out.push({ tag: "🔌", label: "Nuova connessione SSE", azione: () => nuovaConnessione("sse") });
    out.push({ tag: "🧰", label: "Apri Strumenti (JWT, Base64, HMAC, import…)", azione: apriStrumenti });
    out.push({ tag: "◆", label: "Nuova query GraphQL", azione: nuovaGraphQL });
    if (tabAttivo?.tipo === "request") out.push({ tag: "±", label: "Diff modifiche non salvate", azione: diffNonSalvate });
    out.push({ tag: "⌨", label: "Scorciatoie da tastiera", azione: () => (cheatAperto = true) });
    out.push({ tag: "⚡", label: "Svuota cronologia", azione: pulisciStoria });
    // Confronto affiancato: diff della risposta attiva con un altro tab.
    if (tabAttivo?.tipo === "request" && tabAttivo.risposta) {
      for (const t of tabs) {
        if (t.id !== tabAttivoId && t.tipo === "request" && t.risposta) {
          out.push({ tag: "⇄", label: `Confronta risposta con: ${t.richiesta.nome || "(senza nome)"}`,
            azione: () => confrontaRisposte(tabAttivo.risposta.body, t.risposta.body, `${tabAttivo.richiesta.nome} ↔ ${t.richiesta.nome}`) });
        }
      }
    }
    return out;
  });

  function scorciatoieGlobali(e) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      paletteAperta = !paletteAperta;
    }
  }
</script>

<svelte:window onkeydown={scorciatoieGlobali} />

<div class="app">
  <Titlebar />
  <div class="body" style="grid-template-columns: 48px {layout.sidebar}px 5px 1fr">
    <Rail {vista} onCambiaVista={(v) => (vista = v)} />

    <div class="sidebar">
      {#if vista === "collezioni"}
        <CollectionsView
          {albero}
          {percorsoWs}
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
          onGeneraDoc={esportaDoc}
          onEsportaOpenapi={esportaOpenapi}
          onTrovaSostituisci={trovaSostituisci}
          onDrift={confrontaDrift}
          onDiffColl={diffCollezioni}
          onConfigCartella={apriConfigCartella}
          onCoverage={coverageDaSpec}
          onLint={lintDaSpec}
          onEseguiBatch={eseguiBatch}
          onEsportaWs={esportaWs}
          onImportaWs={importaWs}
        />
      {:else if vista === "run"}
        <RunView {albero} onEsegui={eseguiRun} />
      {:else if vista === "storia"}
        <HistoryView
          {storia}
          {runs}
          onApri={apriDaStoria}
          onAggiorna={() => { ricaricaStoria(); ricaricaRuns(); }}
          onPulisci={pulisciStoria}
          onConfronta={confrontaStoria}
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
        {:else if tabAttivo.tipo === "cartella"}
          <FolderConfig
            dir={tabAttivo.dir}
            nome={tabAttivo.nome}
            config={tabAttivo.config}
            onSalva={salvaConfigCartella}
          />
        {:else if tabAttivo.tipo === "socket"}
          <Socket tab={tabAttivo} />
        {:else if tabAttivo.tipo === "strumenti"}
          <Strumenti {environments} onImportaRichiesta={apriRichiestaImportata} />
        {:else if tabAttivo.tipo === "graphql"}
          <GraphQL tab={tabAttivo} variabili={variabiliAttive} />
        {:else if tabAttivo.tipo === "batch"}
          <BatchResults titolo={tabAttivo.titolo} righe={tabAttivo.righe} inCorso={tabAttivo.inCorso}
            ognis={tabAttivo.ognis}
            onPianifica={(s) => pianificaBatch(tabAttivo, s)}
            onFerma={() => fermaBatch(tabAttivo)} />
        {:else}
          <div class="req-area" style="grid-template-columns: 1fr 5px {layout.right}px">
            <div class="editor-col">
              <Editor
                richiesta={tabAttivo.richiesta}
                inCorso={tabAttivo.inCorso}
                salvabile={!!tabAttivo.dir}
                {environments}
                {ambienteAttivo}
                variabili={variabiliAttive}
                onCambiaAmbiente={(f) => (ambienteAttivo = f)}
                onApriEnv={() => (vista = "ambienti")}
                onInvia={invia}
                onSalva={salva}
                onCopiaCodice={copiaCodice}
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
                avvisiSicurezza={tabAttivo.avvisiSicurezza ?? []}
                onCapturaVar={capturaVar}
                onCreaTest={creaTest}
                onAutoTest={autoTest}
                onAutoSchema={autoSchema}
                onSnapshotDiff={snapshotDiff}
                onSnapshotAccetta={snapshotAccetta}
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

{#if paletteAperta}
  <CommandPalette {comandi} onChiudi={() => (paletteAperta = false)} />
{/if}
{#if cheatAperto}
  <CheatSheet onChiudi={() => (cheatAperto = false)} />
{/if}
