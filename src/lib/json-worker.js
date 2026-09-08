// Worker dedicato a formattare/compattare JSON fuori dal thread principale.
// Su un corpo da molti MB `JSON.parse` + `JSON.stringify` bloccherebbero la
// finestra per secondi: qui girano su un thread a parte e la UI resta viva.
self.onmessage = (e) => {
  const { id, azione, testo } = e.data;
  try {
    const dato = JSON.parse(testo);
    const out = azione === "compatta" ? JSON.stringify(dato) : JSON.stringify(dato, null, 2);
    self.postMessage({ id, ok: true, testo: out });
  } catch (err) {
    self.postMessage({ id, ok: false, errore: String(err?.message ?? err) });
  }
};
