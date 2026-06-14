// Versione dell'app e changelog mostrati nella vista Info.

export const VERSIONE = "0.32.0";

// Dal più recente al più vecchio. Il primo è la versione attuale.
export const CHANGELOG = [
  {
    versione: "0.32.0",
    voci: [
      "Visual flow builder: i passi del Run ora hanno condizioni (esegui solo se…), catture di variabili dalla risposta e 'continua al fallimento'.",
      "Risultati Run con passi saltati e variabili catturate.",
    ],
  },
  {
    versione: "0.31.0",
    voci: [
      "Produttività: cookie inspector, variabili di collezione, response come tabella, dirty diff, richieste preferite, filtri per-workspace.",
      "Polish: tema 'sistema' + dimensione font, descrizioni Markdown (tab Note), export/import dell'intero workspace, i18n IT/EN (base).",
    ],
  },
  {
    versione: "0.30.0",
    voci: [
      "GraphQL: console con editor query+variabili, esecuzione e explorer dello schema (introspezione).",
    ],
  },
  {
    versione: "0.29.0",
    voci: [
      "CLI: matrice di ambienti (--envs staging,prod) per eseguire la suite su più ambienti.",
    ],
  },
  {
    versione: "0.28.0",
    voci: [
      "Auto-genera asserzioni dalla risposta (status + campi JSON) con un click.",
      "Mock server dalle snapshot registrate: rustman mock <workspace> (dev offline con dati reali).",
    ],
  },
  {
    versione: "0.27.0",
    voci: [
      "Tema chiaro/scuro e colore d'accento personalizzabile (Settings).",
      "Filtri salvati nell'albero (smart folders) e cheatsheet delle scorciatoie.",
    ],
  },
  {
    versione: "0.26.0",
    voci: [
      "Pannello Strumenti: JWT decoder, base64/URL encode-decode, conversione timestamp, HMAC.",
      "Importa da fetch() o cURL (incolla → richiesta); diff di due ambienti.",
    ],
  },
  {
    versione: "0.25.0",
    voci: [
      "Ricerca/filtro nell'albero (nome, URL, metodo, tag).",
      "Tag sulle richieste; --tag nella CLI per eseguire una suite.",
      "Confronto affiancato delle risposte di due tab (Command Palette).",
      "Scheduler: ripeti un batch a intervalli (monitor leggero).",
    ],
  },
  {
    versione: "0.24.0",
    voci: [
      "Performance: profili di carico spike (picco nella fascia centrale) e soak (lunga durata).",
    ],
  },
  {
    versione: "0.23.0",
    voci: [
      "Security scan delle risposte: header di sicurezza mancanti, CORS aperto, info leak, cookie non sicuri (tab Sicurezza).",
    ],
  },
  {
    versione: "0.22.0",
    voci: [
      "Batch send: esegui un'intera cartella e vedi gli esiti in una griglia (▶ sulle cartelle).",
      "Autocomplete dei {{...}} nell'URL (variabili d'ambiente e dinamiche).",
    ],
  },
  {
    versione: "0.21.0",
    voci: [
      "CI: gate del pass-rate (--min-pass-rate) e flaky detection (--flaky) nella CLI.",
      "Diff di due collezioni esportate (aggiunte/rimosse/modificate).",
    ],
  },
  {
    versione: "0.20.0",
    voci: [
      "Impostazioni per-richiesta: timeout, follow-redirect, verifica TLS, retry su 429.",
      "Rate-limit aware: rispetta Retry-After sui 429; cookie jar di sessione automatico.",
    ],
  },
  {
    versione: "0.19.0",
    voci: [
      "Console WebSocket e SSE: connetti, invia/ricevi messaggi e stream di eventi live.",
    ],
  },
  {
    versione: "0.18.0",
    voci: [
      "Mock server (CLI): serve le risposte d'esempio di uno spec OpenAPI (dev senza backend).",
    ],
  },
  {
    versione: "0.17.0",
    voci: [
      "Trend storico dei test: pass-rate nel tempo nella vista History.",
    ],
  },
  {
    versione: "0.16.0",
    voci: [
      "Export OpenAPI 3.0 dalle collezioni (chiude il cerchio con l'import).",
    ],
  },
  {
    versione: "0.15.0",
    voci: [
      "Code-gen multi-linguaggio: copia come cURL, fetch (JS) o Python (requests).",
    ],
  },
  {
    versione: "0.14.0",
    voci: [
      "Snapshot / golden testing con ignore-paths e approvazione baseline.",
      "Test di carico a durata/RPS con warmup; SLO gate e report HTML nella CLI.",
      "API test coverage dallo spec OpenAPI; variabili faker.",
      "Confronto run, sottocomandi CLI 'perf' e 'coverage'.",
    ],
  },
  {
    versione: "0.13.0",
    voci: [
      "Cattura dal response: campi JSON → variabili o asserzioni con un click.",
      "Trend dei tempi per endpoint nella History (sparkline, avg/p95).",
      "Drift detection fra due spec OpenAPI.",
      "Find & Replace globale ed ereditarietà di header/auth per cartella.",
      "Import HAR; poll/retry nella CLI (--retry/--delay).",
    ],
  },
  {
    versione: "0.12.0",
    voci: [
      "History: confronto (diff) fra due risposte, oltre al replay.",
      "Contract testing: asserzione 'schema' (JSON Schema), auto dall'import OpenAPI.",
      "Run data-driven nella CLI (--data CSV/JSON): un'iterazione per riga.",
      "Command Palette (Ctrl/Cmd+K) per richieste, ambienti, viste e azioni.",
      "Variabili dinamiche ($timestamp, $randomUUID, ...) con anteprima URL.",
      "Generatore di documentazione HTML dalle collezioni.",
    ],
  },
  {
    versione: "0.11.0",
    voci: [
      "OAuth 2.0 (client_credentials/password) con pulsante 'Ottieni token'.",
      "History delle richieste inviate, con replay (riapri e reinvia).",
      "Copia come cURL e import di una richiesta da comando cURL.",
      "La CLI esegue anche gli script pre/post pm.* (motore QuickJS).",
    ],
  },
  {
    versione: "0.10.0",
    voci: [
      "Corpo form-data (con upload file/multipart) e x-www-form-urlencoded.",
      "Variabili segrete tenute fuori da git (.rustman-secrets.json).",
      "Import da OpenAPI/Swagger (3.x e 2.0, JSON o YAML).",
      "CLI headless 'rustman run' per la CI, con report JUnit.",
    ],
  },
  {
    versione: "0.9.0",
    voci: [
      "Import da Postman: Collection v2.x ed Environment.",
      "Mappa cartelle, richieste, header, query, auth, body e script (pm.*).",
      "Le variabili di collezione diventano un ambiente; gli script di collezione/cartella sono ereditati.",
    ],
  },
  {
    versione: "0.8.0",
    voci: [
      "Pannelli ridimensionabili con il mouse (sidebar, editor/risposta, risposta/performance, log).",
      "Vista Info con versione corrente e changelog.",
      "Pannello Log in basso al centro (richieste, errori, script, run).",
    ],
  },
  {
    versione: "0.7.0",
    voci: [
      "Tab Run: catene di chiamate per gli integration test (stop al primo errore).",
      "Pre-script e Post-script in JavaScript con API stile Postman (pm.*).",
      "Gestione ambienti spostata nel menu laterale.",
    ],
  },
  {
    versione: "0.6.0",
    voci: [
      "Interfaccia stile VS Code: barra attività e viste commutabili.",
      "Richieste in più tab; cartelle annidate nelle collezioni.",
      "Impostazioni con autosalvataggio; Git con remote, pull e push.",
      "Workspace multipli (scelta della cartella).",
    ],
  },
  {
    versione: "0.5.0",
    voci: [
      "Environments e variabili {{...}}; query params e autenticazione (Bearer/Basic).",
      "Rinomina/elimina collezioni; import/export delle collezioni.",
      "Versione web (server) oltre al desktop.",
    ],
  },
  {
    versione: "0.1.0 – 0.4.0",
    voci: [
      "Invio richieste HTTP con metriche; collezioni su file.",
      "Versionamento Git (commit, diff, cronologia).",
      "Test/asserzioni sulle risposte; performance test con grafici.",
    ],
  },
];
