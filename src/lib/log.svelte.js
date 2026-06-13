// Log globale dell'app: messaggi mostrati nel pannello in basso.

export const logs = $state([]);

/** Aggiunge una riga di log. livello: "info" | "ok" | "errore". */
export function logga(livello, testo) {
  logs.push({
    ora: new Date().toLocaleTimeString(),
    livello,
    testo: String(testo),
  });
  // Evita che cresca all'infinito.
  if (logs.length > 500) logs.shift();
}

export function pulisciLog() {
  logs.length = 0;
}
