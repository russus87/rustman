// Dimensioni dei pannelli ridimensionabili, persistite in localStorage.

const CHIAVE = "rustman_layout";
const DEFAULT = { sidebar: 252, right: 470, perf: 300, log: 120 };

function carica() {
  try {
    return { ...DEFAULT, ...JSON.parse(localStorage.getItem(CHIAVE) || "{}") };
  } catch {
    return { ...DEFAULT };
  }
}

export const layout = $state(carica());

export function salvaLayout() {
  localStorage.setItem(CHIAVE, JSON.stringify({ ...layout }));
}

// Modifica un valore di layout di `delta` px, mantenendolo entro [min, max].
export function ridimensiona(chiave, delta, min, max) {
  layout[chiave] = Math.max(min, Math.min(max, layout[chiave] + delta));
  salvaLayout();
}
