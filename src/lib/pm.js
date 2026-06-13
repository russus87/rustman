// Motore degli script pre/post-richiesta, in JavaScript, con API stile Postman (`pm.*`).
// Gli script girano nella webview con `new Function` (eval consentito: CSP nulla),
// quindi funzionano identici su desktop e web.

function fmt(x) {
  return typeof x === "string" ? x : JSON.stringify(x);
}

// Costruisce un oggetto `expect` minimale, simile a chai (lancia su mismatch).
function makeExpect(actual) {
  const uguale = (atteso, deep) => {
    const ko = deep
      ? JSON.stringify(actual) !== JSON.stringify(atteso)
      : actual !== atteso;
    if (ko) throw new Error(`atteso ${fmt(atteso)}, ottenuto ${fmt(actual)}`);
  };
  const ok = () => {
    if (!actual) throw new Error(`atteso un valore "ok", ottenuto ${fmt(actual)}`);
  };
  const include = (x) => {
    const dentro =
      typeof actual === "string" || Array.isArray(actual)
        ? actual.includes(x)
        : actual && typeof actual === "object" && x in actual;
    if (!dentro) throw new Error(`${fmt(actual)} non include ${fmt(x)}`);
  };
  return {
    to: {
      equal: (x) => uguale(x, false),
      eql: (x) => uguale(x, true),
      include,
      be: {
        get ok() {
          ok();
          return true;
        },
      },
    },
  };
}

// Crea l'oggetto `pm` e i raccoglitori (log/test) per una esecuzione.
function creaPm({ req, res, vars }) {
  const logs = [];
  const tests = [];
  const consoleProxy = {
    log: (...a) => logs.push(a.map(fmt).join(" ")),
    error: (...a) => logs.push("ERRORE: " + a.map(fmt).join(" ")),
    warn: (...a) => logs.push("ATTENZIONE: " + a.map(fmt).join(" ")),
  };
  const bag = {
    get: (k) => vars[k],
    set: (k, v) => {
      vars[k] = v == null ? "" : String(v);
    },
  };
  const pm = {
    variables: bag,
    environment: bag, // in MVP condividono lo stesso "bag" in memoria
    request: req,
    response: res,
    expect: makeExpect,
    test: (nome, fn) => {
      try {
        fn();
        tests.push({ descrizione: nome, passato: true, dettaglio: "" });
      } catch (e) {
        tests.push({ descrizione: nome, passato: false, dettaglio: String(e?.message ?? e) });
      }
    },
  };
  return { pm, consoleProxy, logs, tests };
}

function esegui(script, ctx) {
  if (!script || !script.trim()) return { logs: [], tests: [] };
  const { pm, consoleProxy, logs, tests } = creaPm(ctx);
  try {
    // eslint-disable-next-line no-new-func
    new Function("pm", "console", script)(pm, consoleProxy);
  } catch (e) {
    logs.push("ERRORE script: " + String(e?.message ?? e));
  }
  return { logs, tests };
}

/** Pre-script: può mutare `req` (oggetto) e `vars` (mappa). */
export function eseguiPre(script, { req, vars }) {
  return esegui(script, { req, res: null, vars });
}

/** Post-script: legge `res` e può scrivere `vars`. */
export function eseguiPost(script, { res, vars }) {
  return esegui(script, { req: null, res, vars });
}

/** Converte una Risposta del backend nell'oggetto `pm.response`. */
export function rispostaToRes(risposta) {
  const headers = {};
  for (const h of risposta.headers || []) headers[h.chiave] = h.valore;
  return {
    code: risposta.status,
    status: risposta.status_text,
    responseTime: risposta.tempo_ms,
    headers,
    text: risposta.body,
    json: () => JSON.parse(risposta.body),
  };
}
