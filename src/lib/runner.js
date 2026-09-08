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

// Configurazione di un nodo: si cerca prima per id di nodo — due nodi possono
// puntare alla stessa richiesta e avere impostazioni diverse — e solo in
// mancanza per file, che è come venivano salvati i flussi più vecchi.
export function indiciPassi(passi) {
  const lista = passi || [];
  return {
    perNodo: new Map(lista.filter((p) => p.nodo).map((p) => [p.nodo, p])),
    perFile: new Map(lista.map((p) => [p.file, p])),
  };
}
export function passoDelNodo(indici, nodo) {
  return indici.perNodo.get(nodo.id) || indici.perFile.get(nodo.file) || null;
}

// Ricostruisce catena.passi dall'ordine topologico del grafo, preservando la
// configurazione esistente (condizione/catture/al_fallimento/ciclo).
export function sincronizzaPassi(catena) {
  if (!catena?.nodi?.length) return catena?.passi || [];
  const indici = indiciPassi(catena.passi);
  const perId = new Map(catena.nodi.map((n) => [n.id, n]));
  return ordineTopologico(catena.nodi, catena.archi || []).map((id) => {
    const nodo = perId.get(id);
    const esistente = passoDelNodo(indici, nodo);
    return esistente
      ? { ...esistente, nodo: id, file: nodo.file }
      : { nodo: id, file: nodo.file, condizione: null, catture: [], al_fallimento: "", ciclo: null };
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

// ---------------------------------------------------------------------------
// Cicli (loop) su un passo
// ---------------------------------------------------------------------------

const pausa = (ms) => new Promise((r) => setTimeout(r, ms));

// Decide quanti giri fare e su quali elementi. Con "foreach" l'array viene
// letto dalla risposta del nodo precedente; se non è un array il passo fallisce
// invece di girare a vuoto, così l'errore si vede nei risultati.
function pianificaCiclo(ciclo, prev) {
  const conc = Math.max(1, Math.trunc(Number(ciclo.concorrenza) || 1));
  if (ciclo.sorgente === "foreach") {
    const lista = prev ? valoreJson(prev.body, ciclo.lista) : undefined;
    if (!Array.isArray(lista)) {
      return { errore: `foreach: "${ciclo.lista || "(nessun path)"}" non è un array nella risposta precedente` };
    }
    return { giri: lista.length, elementi: lista, conc };
  }
  return { giri: Math.max(0, Math.trunc(Number(ciclo.volte) || 0)), elementi: null, conc };
}

// Percentile su una lista già ordinata. Stessa convenzione di
// core/src/perf.rs::percentile, così i numeri del report combaciano con quelli
// del pannello Performance.
function percentile(ordinati, p) {
  if (!ordinati.length) return null;
  const i = Math.round((p / 100) * (ordinati.length - 1));
  return ordinati[Math.min(i, ordinati.length - 1)];
}

// Valore di {{$loopItem}}: gli oggetti passano come JSON, il resto come stringa.
function valoreItem(v) {
  if (v === undefined) return undefined;
  return v !== null && typeof v === "object" ? JSON.stringify(v) : String(v);
}

/**
 * Esegue un passo più volte e restituisce un risultato aggregato.
 *
 * In sequenza le variabili sono condivise fra i giri: uno script che ne scrive
 * una la lascia al giro successivo, e la condizione di uscita viene controllata
 * dopo ogni giro. In concorrenza ogni giro lavora su una copia delle variabili
 * — altrimenti i giri si sovrascriverebbero a vicenda — e le modifiche vengono
 * riportate alla fine in ordine di giro (vince l'ultimo); lì la condizione di
 * uscita può solo impedire l'avvio di nuovi giri, non annullare quelli in volo.
 */
async function eseguiCiclo(passo, albero, vars, prev) {
  const ciclo = passo.ciclo;
  const orig = trovaRichiesta(albero, passo.file);
  const nome = orig?.nome || passo.file;
  if (!orig) {
    return { risultato: { nome: passo.file, ok: false, errore: "richiesta non trovata", tests: [], logs: [] }, risposta: prev };
  }

  const piano = pianificaCiclo(ciclo, prev);
  if (piano.errore) {
    return { risultato: { nome, ok: false, errore: piano.errore, tests: [], logs: [] }, risposta: prev };
  }
  if (piano.giri === 0) {
    return {
      risultato: { nome, saltato: true, ok: true, tests: [], logs: [],
        ciclo: { giri: 0, ok: 0, falliti: 0, concorrenza: piano.conc, uscita: "nessun giro", problemi: [] } },
      risposta: prev,
    };
  }

  // Prepara le variabili di un giro e lo esegue.
  const giroSu = async (i, varsGiro) => {
    varsGiro.$loopIndex = String(i);
    varsGiro.$loopCount = String(piano.giri);
    if (piano.elementi) {
      const it = valoreItem(piano.elementi[i]);
      varsGiro.$loopItem = it === undefined ? "" : it;
    }
    const esito = await eseguiPasso(passo, albero, varsGiro);
    return { i, ...esito };
  };

  const esiti = new Array(piano.giri);
  let uscita = "completato";

  if (piano.conc <= 1) {
    for (let i = 0; i < piano.giri; i++) {
      if (i > 0 && Number(ciclo.ritardo_ms) > 0) await pausa(Number(ciclo.ritardo_ms));
      const e = await giroSu(i, vars);
      esiti[i] = e;
      if (!e.risultato.ok && ciclo.al_fallimento !== "continua") { uscita = "fermato da un giro fallito"; break; }
      if (ciclo.esci_se && valutaCondizione(ciclo.esci_se, e.risposta, vars)) { uscita = "condizione di uscita"; break; }
    }
  } else {
    // Pool di worker: `prossimo` distribuisce gli indici, `stop` impedisce di
    // avviarne altri quando il ciclo deve terminare.
    const copie = new Array(piano.giri);
    let prossimo = 0;
    let stop = false;
    const worker = async () => {
      for (;;) {
        if (stop) return;
        const i = prossimo++;
        if (i >= piano.giri) return;
        const varsGiro = { ...vars };
        copie[i] = varsGiro;
        const e = await giroSu(i, varsGiro);
        esiti[i] = e;
        if (!e.risultato.ok && ciclo.al_fallimento !== "continua") { stop = true; uscita = "fermato da un giro fallito"; return; }
        if (ciclo.esci_se && valutaCondizione(ciclo.esci_se, e.risposta, varsGiro)) { stop = true; uscita = "condizione di uscita"; return; }
      }
    };
    await Promise.all(Array.from({ length: Math.min(piano.conc, piano.giri) }, worker));
    // Riporta le variabili dei giri completati, in ordine di giro.
    for (let i = 0; i < piano.giri; i++) {
      if (!esiti[i] || !copie[i]) continue;
      for (const [k, v] of Object.entries(copie[i])) if (!k.startsWith("$loop")) vars[k] = v;
    }
  }

  const fatti = esiti.filter(Boolean);
  const okN = fatti.filter((e) => e.risultato.ok).length;
  // Latenza di ogni giro, in ordine di giro: serve al report per disegnare
  // andamento e distribuzione. Sono numeri, non corpi: la memoria resta piatta
  // anche su cicli da migliaia di giri.
  const tempi = fatti.map((e) => e.risultato.tempo).filter((t) => typeof t === "number");
  const ordinati = [...tempi].sort((a, b) => a - b);
  const somma = tempi.reduce((a, b) => a + b, 0);
  const problemi = fatti
    .filter((e) => !e.risultato.ok)
    .slice(0, 20)
    .map((e) => ({
      giro: e.i,
      status: e.risultato.status,
      errore: e.risultato.errore,
      test: (e.risultato.tests || []).filter((t) => !t.passato).map((t) => t.descrizione),
    }));

  // Ultimo giro completato: è la risposta che i rami in uscita vedranno.
  let ultima = prev;
  let ultimo = null;
  for (let i = piano.giri - 1; i >= 0; i--) {
    if (esiti[i]) { ultimo = esiti[i]; if (esiti[i].risposta) ultima = esiti[i].risposta; break; }
  }

  return {
    risultato: {
      nome,
      ok: fatti.length > 0 && okN === fatti.length,
      status: ultimo?.risultato.status,
      tempo: somma,
      tests: [],
      logs: ultimo?.risultato.logs || [],
      catture: ultimo?.risultato.catture,
      ciclo: {
        giri: fatti.length,
        previsti: piano.giri,
        ok: okN,
        falliti: fatti.length - okN,
        concorrenza: piano.conc,
        sorgente: ciclo.sorgente === "foreach" ? `foreach ${ciclo.lista}` : "volte",
        tempoTot: somma,
        tempoMedio: tempi.length ? Math.round(somma / tempi.length) : null,
        tempoMin: ordinati.length ? ordinati[0] : null,
        tempoMax: ordinati.length ? ordinati[ordinati.length - 1] : null,
        p50: percentile(ordinati, 50),
        p90: percentile(ordinati, 90),
        p95: percentile(ordinati, 95),
        p99: percentile(ordinati, 99),
        tempi,
        testOk: fatti.reduce((n, e) => n + (e.risultato.tests || []).filter((t) => t.passato).length, 0),
        testTot: fatti.reduce((n, e) => n + (e.risultato.tests || []).length, 0),
        uscita,
        problemi,
      },
    },
    risposta: ultima,
  };
}

// Esegue un passo, ripetendolo se ha un ciclo configurato.
function eseguiPassoOCiclo(passo, albero, vars, prev) {
  return passo.ciclo ? eseguiCiclo(passo, albero, vars, prev) : eseguiPasso(passo, albero, vars);
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
    const { risultato, risposta } = await eseguiPassoOCiclo(passo, albero, vars, prev);
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
  const indici = indiciPassi(catena.passi);
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
      const passo = passoDelNodo(indici, nodo)
        || { nodo: nodo.id, file: nodo.file, condizione: null, catture: [], al_fallimento: "", ciclo: null };
      // Skip del nodo: condizione valutata sulla risposta del predecessore.
      if (!valutaCondizione(passo.condizione, predResp.get(id) || null, vars)) {
        risultati.push({ nome: nodo.label || nodo.file, saltato: true, ok: true, tests: [], logs: [] });
        continue; // nodo saltato: non propaga oltre
      }
      const esito = await eseguiPassoOCiclo(passo, albero, vars, predResp.get(id) || null);
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
