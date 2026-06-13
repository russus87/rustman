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

/**
 * Esegue la catena. `varsBase` è la mappa delle variabili dell'ambiente attivo.
 * Restituisce un array di risultati per passo.
 */
export async function eseguiCatena(catena, albero, varsBase) {
  const vars = { ...(varsBase || {}) };
  const risultati = [];

  for (const passo of catena.passi) {
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
      risposta = await api.inviaRichiesta(req, vars);
    } catch (e) {
      errore = String(e);
    }

    let tests = [];
    if (risposta) {
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
    });

    if (!ok) break; // stop al primo errore
  }

  return risultati;
}
