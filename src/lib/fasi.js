// Fasi dichiarate dal servizio dentro il corpo della risposta.
//
// Molte API raccontano quanto è durata ogni tappa della loro elaborazione, di
// solito in un array di log:
//
//   { "log": ["Step: LoadFontStep completed in 349.9064 ms",
//             "Step: ToPsProcessStep completed in 1822.2507 ms"] }
//
// Qui quelle righe tornano a essere numeri, così il pannello Response può dire
// quanto pesa ogni fase sul totale. Se nel corpo non c'è niente di
// riconoscibile la lista è vuota e non si mostra nulla: non si inventano fasi.
//
// Le stesse regole sono implementate in `core/src/fasi.rs`, che le applica alle
// risposte del test di carico (lì i corpi non arrivano mai al frontend). Se
// cambi cosa viene riconosciuto, cambialo in tutti e due i posti.

// Parole che separano il nome della fase dalla sua durata.
const SEPARATORI = [
  "completed in",
  "completata in",
  "completato in",
  "finished in",
  "elapsed in",
  "eseguita in",
  "took",
];

// Prefissi che introducono il nome della fase in una riga di log: tutto ciò che
// li precede (timestamp, livello di log) non fa parte del nome.
const PREFISSI = ["step:", "fase:", "phase:", "task:"];

// Chiavi che, in un oggetto JSON, portano il nome della fase.
const CHIAVI_NOME = ["step", "fase", "phase", "tappa", "task", "name", "nome", "label", "etichetta"];

// Chiavi di durata → quanti millisecondi vale il loro valore.
const CHIAVI_DURATA = {
  ms: 1, millis: 1, milliseconds: 1, millisecondi: 1, durationms: 1, duratams: 1,
  elapsedms: 1, tempoms: 1, timems: 1, tookms: 1, duration: 1, durata: 1, elapsed: 1, took: 1,
  s: 1000, sec: 1000, seconds: 1000, secondi: 1000, durations: 1000, duratas: 1000,
  durationseconds: 1000, elapsedseconds: 1000,
};

// Unità scritte accanto al numero → millisecondi.
const UNITA = {
  ms: 1, msec: 1, millis: 1, millisecondi: 1, milliseconds: 1,
  s: 1000, sec: 1000, secs: 1000, second: 1000, seconds: 1000, secondi: 1000,
  us: 0.001, "µs": 0.001, microsecondi: 0.001, microseconds: 0.001,
  ns: 0.000001,
  m: 60000, min: 60000, minuti: 60000, minutes: 60000,
};

// Quante fasi al massimo si raccolgono da una risposta: oltre non servono a
// leggere niente e attraversare il resto del JSON costerebbe soltanto.
const MAX_FASI = 500;

// Riconosce una riga tipo "Step: LoadFontStep completed in 349.9064 ms".
export function faseDaRiga(riga) {
  if (typeof riga !== "string") return null;
  const basso = riga.toLowerCase();
  let pos = -1, sep = "";
  for (const s of SEPARATORI) {
    const p = basso.indexOf(s);
    if (p >= 0 && (pos < 0 || p < pos)) { pos = p; sep = s; }
  }
  if (pos < 0) return null;
  const ms = durataMs(riga.slice(pos + sep.length).trimStart());
  if (ms === null) return null;

  // Il nome è quello che segue "Step:" (se c'è), altrimenti tutto quello che
  // precede il separatore.
  let inizio = 0;
  for (const p of PREFISSI) {
    const i = basso.lastIndexOf(p, pos);
    if (i >= 0 && i < pos) inizio = Math.max(inizio, i + p.length);
  }
  const nome = riga.slice(inizio, pos).trim().replace(/^["',|-]+|["',|-]+$/g, "").trim();
  if (!nome || nome.length > 120) return null;
  return { nome, ms };
}

// Riconosce un oggetto tipo { "step": "LoadFont", "durationMs": 349.9 }.
function faseDaOggetto(o) {
  let nome = null, ms = null;
  for (const [k, v] of Object.entries(o)) {
    const key = k.toLowerCase().replace(/[_\- ]/g, "");
    if (nome === null && CHIAVI_NOME.includes(key) && typeof v === "string") nome = v.trim();
    if (ms === null && CHIAVI_DURATA[key] !== undefined && typeof v === "number") {
      ms = v * CHIAVI_DURATA[key];
    }
  }
  if (!nome || nome.length > 120) return null;
  if (ms === null || !Number.isFinite(ms) || ms < 0) return null;
  return { nome, ms };
}

// Legge una durata con la sua unità: "349.9064 ms" → 349.9064, "1,5 s" → 1500.
// Senza unità restituisce null: un numero da solo non è una durata, e prenderlo
// per tale riempirebbe l'elenco di fasi inventate.
function durataMs(t) {
  const grezzo = (t.match(/^[\d.,]+/) || [""])[0];
  const numero = grezzo.replace(/[.,]+$/, "");
  if (!numero) return null;
  // "1.234,56" e "1,234.56": il separatore che compare per ultimo è quello dei
  // decimali, l'altro raggruppa le migliaia e va tolto.
  const punto = numero.lastIndexOf("."), virgola = numero.lastIndexOf(",");
  let pulito;
  if (punto >= 0 && virgola >= 0) {
    pulito = punto > virgola ? numero.replace(/,/g, "") : numero.replace(/\./g, "").replace(",", ".");
  } else {
    pulito = numero.replace(",", ".");
  }
  const valore = Number(pulito);
  if (!Number.isFinite(valore)) return null;
  const unita = (t.slice(grezzo.length).trimStart().match(/^[a-zA-Zµ]+/) || [""])[0].toLowerCase();
  const fattore = UNITA[unita];
  return fattore === undefined ? null : valore * fattore;
}

// Attraversa un JSON già analizzato raccogliendo le fasi, in ordine di lettura.
export function fasiDaJson(valore) {
  const out = [];
  cammina(valore, out);
  return out;
}

function cammina(v, out) {
  if (out.length >= MAX_FASI) return;
  if (typeof v === "string") {
    const f = faseDaRiga(v);
    if (f) out.push(f);
  } else if (Array.isArray(v)) {
    for (const x of v) cammina(x, out);
  } else if (v && typeof v === "object") {
    // Un oggetto che è già una fase non viene riaperto: i suoi campi sono il
    // nome e la durata che abbiamo appena letto.
    const f = faseDaOggetto(v);
    if (f) out.push(f);
    else for (const x of Object.values(v)) cammina(x, out);
  }
}

// Legge un corpo non JSON riga per riga, come se fosse un log.
export function fasiDaTesto(testo) {
  const out = [];
  for (const riga of testo.split("\n")) {
    const f = faseDaRiga(riga);
    if (f) out.push(f);
    if (out.length >= MAX_FASI) break;
  }
  return out;
}

// Somma delle fasi, quota di ciascuna e tempo non spiegato da nessuna di esse
// (rete, coda, serializzazione): è quello che resta del tempo di risposta.
export function riepilogo(fasi, tempoTotaleMs = 0) {
  const somma = fasi.reduce((s, f) => s + f.ms, 0);
  const righe = fasi.map((f) => ({ ...f, quota: somma > 0 ? (f.ms / somma) * 100 : 0 }));
  const piuLenta = righe.reduce((a, b) => (b.ms > (a?.ms ?? -1) ? b : a), null);
  // Il tempo misurato dal client include la rete: sotto la somma delle fasi non
  // può stare, e se ci sta (orologi diversi, misure parziali) non si mostra.
  const altro = tempoTotaleMs > somma ? tempoTotaleMs - somma : 0;
  return { somma, righe, altro, piuLenta };
}

// Millisecondi in forma leggibile: 349,9 ms / 1,82 s.
export function fmtMs(ms) {
  if (ms >= 1000) return `${(ms / 1000).toLocaleString("it-IT", { maximumFractionDigits: 2 })} s`;
  if (ms >= 10) return `${ms.toLocaleString("it-IT", { maximumFractionDigits: 1 })} ms`;
  return `${ms.toLocaleString("it-IT", { maximumFractionDigits: 3 })} ms`;
}
