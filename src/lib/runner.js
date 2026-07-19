// Esecutore di catene di chiamate (integration test).
// Per ogni passo: pre-script → invio → post-script + asserzioni dichiarative.
// Le variabili passano da un passo al successivo. Si ferma al primo errore.

import * as api from "./api.js";
import { eseguiPre, eseguiPost, rispostaToRes } from "./pm.js";

// ---------------------------------------------------------------------------
// Grafo del flusso (workflow visuale). I `passi` restano il formato lineare
// usato dalla CLI; questi helper convertono da/verso il grafo `nodi`/`archi`.
// ---------------------------------------------------------------------------

// Ordine topologico di TUTTI i nodi (Kahn). I cicli/isolati sono appesi in coda.
export function ordineDiVisita(nodi, archi) {
  const perId = new Map(nodi.map((n) => [n.id, n]));
  const uscite = new Map(nodi.map((n) => [n.id, []]));
  const gradoIn = new Map(nodi.map((n) => [n.id, 0]));
  for (const a of archi || []) {
    if (!perId.has(a.da) || !perId.has(a.a)) continue;
    uscite.get(a.da).push(a.a);
    gradoIn.set(a.a, gradoIn.get(a.a) + 1);
  }
  const coda = nodi.filter((n) => (gradoIn.get(n.id) || 0) === 0).map((n) => n.id);
  const visti = new Set();
  const ordine = [];
  while (coda.length) {
    const id = coda.shift();
    if (visti.has(id)) continue;
    visti.add(id);
    ordine.push(id);
    for (const succ of uscite.get(id) || []) {
      gradoIn.set(succ, gradoIn.get(succ) - 1);
      if ((gradoIn.get(succ) || 0) <= 0) coda.push(succ);
    }
  }
  for (const n of nodi) if (!visti.has(n.id)) ordine.push(n.id);
  return ordine;
}

// Come sopra ma limitato ai soli nodi "request" (per la linearizzazione CLI).
export function ordineTopologico(nodi, archi) {
  const perId = new Map(nodi.map((n) => [n.id, n]));
  return ordineDiVisita(nodi, archi).filter((id) => {
    const n = perId.get(id);
    return n && (n.tipo || "request") === "request" && n.file;
  });
}

// Ricostruisce catena.passi dall'ordine topologico del grafo, preservando la
// configurazione esistente (condizione/catture/al_fallimento) abbinata per file.
export function sincronizzaPassi(catena) {
  if (!catena?.nodi?.length) return catena?.passi || [];
  const perFile = new Map((catena.passi || []).map((p) => [p.file, p]));
  const perId = new Map(catena.nodi.map((n) => [n.id, n]));
  return ordineTopologico(catena.nodi, catena.archi || []).map((id) => {
    const file = perId.get(id).file;
    return perFile.get(file) || { file, condizione: null, catture: [], al_fallimento: "" };
  });
}

// Cerca una richiesta nell'albero dato il percorso file.
export function trovaRichiesta(albero, file) {
  const cerca = (figli) => {
    for (const n of figli) {
      if (n.tipo === "richiesta" && n.file === file) return n.richiesta;
      if (n.tipo === "cartella") {
        const r = cerca(n.figli);
        if (r) return r;
      }
    }
    return null;
  };
  for (const c of albero) {
    const r = cerca(c.figli);
    if (r) return r;
  }
  return null;
}

// Naviga un body JSON con path puntato (es. "data.items.0.id").
function valoreJson(body, path) {
  try {
    let cur = JSON.parse(body);
    for (const p of String(path).split(".")) {
      if (p === "") continue;
      if (cur == null) return undefined;
      cur = Array.isArray(cur) ? cur[Number(p)] : cur[p];
    }
    return cur;
  } catch {
    return undefined;
  }
}

function confronta(ottenuto, operatore, atteso) {
  const o = String(ottenuto ?? "").trim();
  const a = String(atteso ?? "").trim();
  switch (operatore) {
    case "==": return o === a;
    case "!=": return o !== a;
    case "contiene": return o.includes(a);
    case "<": return Number(o) < Number(a);
    case ">": return Number(o) > Number(a);
    default: return false;
  }
}

// Valuta la condizione di un passo sulla risposta precedente / variabili.
function valutaCondizione(cond, prev, vars) {
  if (!cond) return true;
  let ottenuto;
  if (cond.tipo === "status") ottenuto = prev?.status;
  else if (cond.tipo === "var") ottenuto = vars[cond.campo];
  else if (cond.tipo === "json") ottenuto = prev ? valoreJson(prev.body, cond.campo) : undefined;
  return confronta(ottenuto, cond.operatore, cond.atteso);
}

// Estrae i valori indicati dalla risposta e li salva nelle variabili.
function applicaCatture(catture, risposta, vars) {
  const fatte = {};
  for (const c of catture || []) {
    if (!c.variabile) continue;
    let v;
    if (c.da === "header") v = (risposta.headers || []).find((h) => h.chiave.toLowerCase() === String(c.campo).toLowerCase())?.valore;
    else if (c.da === "body") v = risposta.body;
    else v = valoreJson(risposta.body, c.campo);
    if (v !== undefined) {
      const s = typeof v === "object" ? JSON.stringify(v) : String(v);
      vars[c.variabile] = s;
      fatte[c.variabile] = s;
    }
  }
  return fatte;
}

/**
 * Esegue il flusso. `varsBase` è la mappa delle variabili dell'ambiente attivo.
 * Supporta condizioni (salta il passo), catture (salva variabili) e
 * "continua" al fallimento. Restituisce un array di risultati per passo.
 */
export async function eseguiCatena(catena, albero, varsBase) {
  const vars = { ...(varsBase || {}) };
  const risultati = [];
  let prev = null; // risposta del passo precedente (per le condizioni)

  for (const passo of catena.passi) {
    // Condizione: se falsa, salta il passo (non interrompe il flusso).
    if (!valutaCondizione(passo.condizione, prev, vars)) {
      risultati.push({ nome: passo.file, saltato: true, ok: true, tests: [], logs: [] });
      continue;
    }
    const { risultato, risposta } = await eseguiPasso(passo, albero, vars);
    if (risposta) prev = risposta;
    risultati.push(risultato);
    if (risultato.errore === "richiesta non trovata") break;
    // Stop al fallimento, a meno che il passo non sia marcato "continua".
    if (!risultato.ok && passo.al_fallimento !== "continua") break;
  }

  return risultati;
}

// Esegue un singolo passo: pre-script → invio → test + post-script → catture.
// Ritorna { risultato, risposta }; è il chiamante a decidere se fermarsi.
async function eseguiPasso(passo, albero, vars) {
  const orig = trovaRichiesta(albero, passo.file);
  if (!orig) {
    return { risultato: { nome: passo.file, ok: false, errore: "richiesta non trovata", tests: [], logs: [] }, risposta: null };
  }
  // Copia di lavoro (gli script non alterano la richiesta salvata).
  const req = JSON.parse(JSON.stringify(orig));
  const logs = [];

  const pre = eseguiPre(req.pre_script, { req, vars });
  logs.push(...pre.logs);

  let risposta = null;
  let errore = null;
  try {
    risposta = await api.inviaRichiesta(req, vars, passo.file.slice(0, passo.file.lastIndexOf("/")));
  } catch (e) {
    errore = String(e);
  }

  let tests = [];
  let catture = {};
  if (risposta) {
    if (req.tests?.length) {
      try { tests = await api.valutaTest(req.tests, risposta); } catch { /* ignora */ }
    }
    const post = eseguiPost(req.post_script, { res: rispostaToRes(risposta), vars });
    logs.push(...post.logs);
    tests = [...tests, ...post.tests];
    catture = applicaCatture(passo.catture, risposta, vars);
  }

  const testKo = tests.some((t) => !t.passato);
  const ok = !!risposta && !errore && !testKo;
  return {
    risultato: { nome: req.nome || passo.file, ok, errore, status: risposta?.status, tempo: risposta?.tempo_ms, tests, logs, catture },
    risposta,
  };
}

/**
 * Esegue il flusso come GRAFO: attiva i nodi radice, fa girare ogni nodo
 * raggiunto una volta sola (merge) e propaga lungo gli archi la cui condizione
 * è soddisfatta sulla risposta del nodo sorgente (branch). La config del nodo
 * (skip/catture/al_fallimento) è abbinata al passo per file.
 */
export async function eseguiGrafo(catena, albero, varsBase) {
  const nodi = catena?.nodi || [];
  const archi = catena?.archi || [];
  if (!nodi.length) return eseguiCatena(catena, albero, varsBase);

  const vars = { ...(varsBase || {}) };
  const perId = new Map(nodi.map((n) => [n.id, n]));
  const perFile = new Map((catena.passi || []).map((p) => [p.file, p]));
  const uscite = new Map(nodi.map((n) => [n.id, []]));
  const gradoIn = new Map(nodi.map((n) => [n.id, 0]));
  for (const a of archi) {
    if (!perId.has(a.da) || !perId.has(a.a)) continue;
    uscite.get(a.da).push(a);
    gradoIn.set(a.a, (gradoIn.get(a.a) || 0) + 1);
  }

  const attivo = new Set();
  const predResp = new Map(); // risposta del predecessore che ha attivato il nodo
  for (const n of nodi) if ((gradoIn.get(n.id) || 0) === 0) attivo.add(n.id);

  const risultati = [];
  let interrotto = false;

  for (const id of ordineDiVisita(nodi, archi)) {
    const nodo = perId.get(id);
    if (!nodo) continue;
    if (interrotto) break;
    const isStart = (nodo.tipo || "request") === "start";

    if (!attivo.has(id)) {
      if (!isStart) risultati.push({ nome: nodo.label || nodo.file, saltato: true, ok: true, tests: [], logs: [] });
      continue;
    }

    let resp = predResp.get(id) || null;
    if (!isStart) {
      const passo = perFile.get(nodo.file) || { file: nodo.file, condizione: null, catture: [], al_fallimento: "" };
      // Skip del nodo: condizione valutata sulla risposta del predecessore.
      if (!valutaCondizione(passo.condizione, predResp.get(id) || null, vars)) {
        risultati.push({ nome: nodo.label || nodo.file, saltato: true, ok: true, tests: [], logs: [] });
        continue; // nodo saltato: non propaga oltre
      }
      const esito = await eseguiPasso(passo, albero, vars);
      resp = esito.risposta;
      risultati.push(esito.risultato);
      if (esito.risultato.errore === "richiesta non trovata") { interrotto = true; continue; }
      if (!esito.risultato.ok && passo.al_fallimento !== "continua") { interrotto = true; continue; }
    }

    // Propaga lungo gli archi la cui condizione è soddisfatta (branch).
    for (const a of uscite.get(id) || []) {
      if (a.condizione && !valutaCondizione(a.condizione, resp, vars)) continue;
      attivo.add(a.a);
      if (!predResp.has(a.a)) predResp.set(a.a, resp);
    }
  }

  return risultati;
}
