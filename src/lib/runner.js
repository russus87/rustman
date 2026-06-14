// Esecutore di catene di chiamate (integration test).
// Per ogni passo: pre-script → invio → post-script + asserzioni dichiarative.
// Le variabili passano da un passo al successivo. Si ferma al primo errore.

import * as api from "./api.js";
import { eseguiPre, eseguiPost, rispostaToRes } from "./pm.js";

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

    const orig = trovaRichiesta(albero, passo.file);
    if (!orig) {
      risultati.push({ nome: passo.file, ok: false, errore: "richiesta non trovata", tests: [], logs: [] });
      break;
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
      prev = risposta;
      if (req.tests?.length) {
        try {
          tests = await api.valutaTest(req.tests, risposta);
        } catch {
          /* ignora */
        }
      }
      const post = eseguiPost(req.post_script, { res: rispostaToRes(risposta), vars });
      logs.push(...post.logs);
      tests = [...tests, ...post.tests];
      catture = applicaCatture(passo.catture, risposta, vars);
    }

    const testKo = tests.some((t) => !t.passato);
    const ok = !!risposta && !errore && !testKo;
    risultati.push({
      nome: req.nome || passo.file,
      ok,
      errore,
      status: risposta?.status,
      tempo: risposta?.tempo_ms,
      tests,
      logs,
      catture,
    });

    // Stop al fallimento, a meno che il passo non sia marcato "continua".
    if (!ok && passo.al_fallimento !== "continua") break;
  }

  return risultati;
}
