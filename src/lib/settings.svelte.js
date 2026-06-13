// Impostazioni dell'app, reattive e persistite in localStorage.
// Funziona sia su desktop (Tauri) sia su web.

const CHIAVE = "rustman_settings";
const DEFAULT = { autosave: false, autosaveMs: 1000 };

function carica() {
  try {
    return { ...DEFAULT, ...JSON.parse(localStorage.getItem(CHIAVE) || "{}") };
  } catch {
    return { ...DEFAULT };
  }
}

// Stato reattivo condiviso tra tutti i componenti che lo importano.
export const settings = $state(carica());

// Salva le impostazioni correnti (da chiamare dopo una modifica).
export function salvaSettings() {
  localStorage.setItem(
    CHIAVE,
    JSON.stringify({ autosave: settings.autosave, autosaveMs: settings.autosaveMs }),
  );
}
