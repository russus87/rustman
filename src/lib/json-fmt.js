// Formattazione JSON delegata a un worker (vedi json-worker.js).
// Il worker viene creato pigramente al primo uso e poi riusato: crearlo ogni
// volta costerebbe piu' del lavoro stesso sui corpi piccoli.
let worker = null;
let prossimo = 0;
const attesa = new Map();

function avvia() {
  if (worker || typeof Worker === "undefined") return worker;
  try {
    worker = new Worker(new URL("./json-worker.js", import.meta.url), { type: "module" });
    worker.onmessage = (e) => {
      const { id, ok, testo, errore } = e.data;
      const p = attesa.get(id);
      if (!p) return;
      attesa.delete(id);
      ok ? p.risolvi(testo) : p.rifiuta(new Error(errore));
    };
    worker.onerror = () => {
      // Il worker e' morto: sblocca le richieste in attesa e torna al fallback.
      for (const p of attesa.values()) p.rifiuta(new Error("worker non disponibile"));
      attesa.clear();
      worker = null;
    };
  } catch {
    worker = null;
  }
  return worker;
}

// Restituisce il testo indentato (azione "formatta") o su una riga sola
// ("compatta"). Rifiuta se il testo non e' JSON valido.
export function trasformaJson(testo, azione = "formatta") {
  const w = avvia();
  if (!w) {
    // Fallback sincrono (worker non disponibile): accettabile, i corpi grandi
    // sono l'eccezione e senza worker non c'e' alternativa.
    const dato = JSON.parse(testo);
    return Promise.resolve(azione === "compatta" ? JSON.stringify(dato) : JSON.stringify(dato, null, 2));
  }
  const id = ++prossimo;
  return new Promise((risolvi, rifiuta) => {
    attesa.set(id, { risolvi, rifiuta });
    w.postMessage({ id, azione, testo });
  });
}
