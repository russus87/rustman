// Impostazioni dell'app, reattive e persistite in localStorage.
// Funziona sia su desktop (Tauri) sia su web.

const CHIAVE = "rustman_settings";
const DEFAULT = { autosave: false, autosaveMs: 1000, tema: "scuro", accento: "#7c5cff", scala: 1, lingua: "it" };

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
    JSON.stringify({
      autosave: settings.autosave,
      autosaveMs: settings.autosaveMs,
      tema: settings.tema,
      accento: settings.accento,
      scala: settings.scala,
      lingua: settings.lingua,
    }),
  );
}

// ---- Utilità colore (per derivare le varianti dell'accento a runtime) ----

// "#rgb" o "#rrggbb" → {r,g,b}. Ritorna il viola di default se non valido.
function hexRgb(hex) {
  let h = String(hex || "").replace("#", "").trim();
  if (h.length === 3) h = h.split("").map((c) => c + c).join("");
  const n = parseInt(h, 16);
  if (h.length !== 6 || Number.isNaN(n)) return { r: 124, g: 92, b: 255 };
  return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255 };
}
function toHex({ r, g, b }) {
  const c = (v) => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, "0");
  return `#${c(r)}${c(g)}${c(b)}`;
}
// Miscela lineare: t=0 → a, t=1 → b.
function mescola(a, b, t) {
  return { r: a.r + (b.r - a.r) * t, g: a.g + (b.g - a.g) * t, b: a.b + (b.b - a.b) * t };
}
// Luminanza relativa (WCAG) per scegliere il colore di contrasto su fondo accento.
function luminanza({ r, g, b }) {
  const f = (v) => {
    v /= 255;
    return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
}

// Applica tema, accento (con varianti derivate) e scala (zoom) al documento.
export function applicaTema() {
  const root = document.documentElement;
  let chiaro = settings.tema === "chiaro";
  if (settings.tema === "sistema") {
    chiaro = window.matchMedia && window.matchMedia("(prefers-color-scheme: light)").matches;
  }
  root.dataset.tema = chiaro ? "light" : "dark";

  // Dall'unico colore scelto deriviamo l'intera scala d'accento, così cambiare
  // brand a runtime aggiorna gradienti, badge tenui e contrasto in modo coerente.
  const base = hexRgb(settings.accento);
  const bianco = { r: 255, g: 255, b: 255 };
  const nero = { r: 0, g: 0, b: 0 };
  root.style.setProperty("--accent", settings.accento);
  root.style.setProperty("--accent-2", toHex(mescola(base, bianco, 0.16))); // fine gradiente
  root.style.setProperty("--accent-strong", toHex(mescola(base, nero, 0.14))); // stato premuto
  root.style.setProperty("--accent-soft", `rgba(${base.r},${base.g},${base.b},.14)`); // sfondo tenue
  root.style.setProperty("--accent-line", `rgba(${base.r},${base.g},${base.b},.35)`); // bordi tinti
  root.style.setProperty("--accent-contrast", luminanza(base) > 0.45 ? "#12121a" : "#ffffff");

  root.style.zoom = String(settings.scala || 1);
}
