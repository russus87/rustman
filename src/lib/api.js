// Astrazione del "trasporto": isola il frontend da COME si parla col backend.
// - Su desktop (Tauri) usa i comandi nativi via `invoke`.
// - Sul web usa fetch verso il server Axum (/api/<comando>).
// La UI è identica nei due casi: cambia solo questo file.

// Rileva a runtime se siamo dentro l'app Tauri.
const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
let _invoke;

// Chiama un comando del backend con un oggetto di argomenti.
async function call(comando, args = {}) {
  if (isTauri) {
    // Import dinamico: su web il modulo Tauri non viene mai caricato.
    _invoke ??= (await import("@tauri-apps/api/core")).invoke;
    return _invoke(comando, args);
  }
  const res = await fetch(`/api/${comando}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(args),
  });
  if (!res.ok) throw new Error(await res.text());
  const testo = await res.text();
  return testo ? JSON.parse(testo) : null;
}

// ---- HTTP ----
export const inviaRichiesta = (richiesta, variabili) =>
  call("invia_richiesta", { richiesta, variabili: variabili ?? null });

// ---- OAuth2 ----
export const oauth2Token = (auth, variabili) =>
  call("oauth2_token", { auth, variabili: variabili ?? null });

// ---- cURL ----
export const generaCurl = (richiesta) => call("genera_curl", { richiesta });
export const importaCurl = (comando) => call("importa_curl", { comando });

// ---- History ----
export const caricaStoria = () => call("carica_storia");
export const aggiungiStoria = (voce) => call("aggiungi_storia", { voce });
export const pulisciStoria = () => call("pulisci_storia");

// ---- Test ----
export const valutaTest = (asserzioni, risposta) =>
  call("valuta_test", { asserzioni, risposta });

// ---- Performance ----
export const eseguiPerf = (richiesta, n, concorrenza, variabili) =>
  call("esegui_perf", { richiesta, n, concorrenza, variabili: variabili ?? null });

// ---- Collection / cartelle ----
export const percorsoWorkspace = () => call("percorso_workspace");
export const caricaAlbero = () => call("carica_albero");
export const salvaRichiesta = (dir, filePrecedente, richiesta) =>
  call("salva_richiesta", { dir, filePrecedente, richiesta });
export const creaCollezione = (nome) => call("crea_collezione", { nome });
export const creaCartella = (dirGenitore, nome) =>
  call("crea_cartella", { dirGenitore, nome });
export const creaRichiesta = (dir, nome) => call("crea_richiesta", { dir, nome });
export const elimina = (file) => call("elimina", { file });
export const rinominaCartella = (dir, nuovoNome) =>
  call("rinomina_cartella", { dir, nuovoNome });
export const eliminaCartella = (dir) => call("elimina_cartella", { dir });

// ---- Workspaces ----
export const listaWorkspaces = () => call("lista_workspaces");
export const aggiungiWorkspace = (percorso) => call("aggiungi_workspace", { percorso });
export const impostaWorkspaceAttivo = (percorso) =>
  call("imposta_workspace_attivo", { percorso });
export const rimuoviWorkspace = (percorso) => call("rimuovi_workspace", { percorso });

// ---- Import / Export ----
export const esportaCollezione = (dir) => call("esporta_collezione", { dir });
export const importaCollezione = (contenuto) =>
  call("importa_collezione", { contenuto });
// Import "smart": riconosce sia il formato Rustman sia quello Postman
// (collection o environment). Restituisce { tipo: "collezione"|"environment", ... }.
export const importa = (contenuto) => call("importa", { contenuto });

// ---- Catene di run ----
export const caricaCatene = () => call("carica_catene");
export const salvaCatena = (filePrecedente, catena) =>
  call("salva_catena", { filePrecedente, catena });
export const eliminaCatena = (file) => call("elimina_catena", { file });

// ---- Environments ----
export const caricaEnvironments = () => call("carica_environments");
export const salvaEnvironment = (filePrecedente, environment) =>
  call("salva_environment", { filePrecedente, environment });
export const eliminaEnvironment = (file) => call("elimina_environment", { file });

// ---- Git ----
export const gitStato = () => call("git_stato");
export const gitDiff = (file) => call("git_diff", { file });
export const gitCommit = (messaggio, files) => call("git_commit", { messaggio, files });
export const gitLog = () => call("git_log");
export const gitInfo = () => call("git_info");
export const gitImpostaRemote = (url) => call("git_imposta_remote", { url });
export const gitPull = () => call("git_pull");
export const gitPush = () => call("git_push");

// Rileva se siamo in Tauri (per usare il folder picker nativo).
export const inTauri = isTauri;
