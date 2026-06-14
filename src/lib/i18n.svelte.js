// Internazionalizzazione minimale: t(chiave) restituisce la stringa nella lingua
// scelta nei settings. Base da estendere ai vari componenti.
import { settings } from "./settings.svelte.js";

const dict = {
  it: {
    Collections: "Collezioni", Run: "Run", History: "Cronologia", Git: "Git",
    Environments: "Ambienti", Workspaces: "Workspace", Settings: "Impostazioni", Info: "Info",
    Tema: "Tema", Accento: "Accento", Dimensione: "Dimensione", Lingua: "Lingua",
    Scuro: "Scuro", Chiaro: "Chiaro", Sistema: "Sistema",
    Autosalvataggio: "Autosalvataggio",
    "Nuovo ambiente": "Nuovo ambiente", Variabili: "Variabili", Nome: "Nome",
    Salva: "Salva", Attiva: "Attiva", Attivo: "Attivo", Elimina: "Elimina",
    "Nessun ambiente.": "Nessun ambiente.", Passi: "Passi", Esegui: "Esegui",
    Strumenti: "Strumenti", Preferiti: "Preferiti",
  },
  en: {
    Collections: "Collections", Run: "Run", History: "History", Git: "Git",
    Environments: "Environments", Workspaces: "Workspaces", Settings: "Settings", Info: "Info",
    Tema: "Theme", Accento: "Accent", Dimensione: "Size", Lingua: "Language",
    Scuro: "Dark", Chiaro: "Light", Sistema: "System",
    Autosalvataggio: "Autosave",
    "Nuovo ambiente": "New environment", Variabili: "Variables", Nome: "Name",
    Salva: "Save", Attiva: "Activate", Attivo: "Active", Elimina: "Delete",
    "Nessun ambiente.": "No environments.", Passi: "Steps", Esegui: "Run",
    Strumenti: "Tools", Preferiti: "Favorites",
  },
};

// Reattivo: legge settings.lingua, quindi i template che usano t() si aggiornano.
export function t(chiave) {
  const l = settings.lingua === "en" ? "en" : "it";
  return dict[l][chiave] ?? dict.it[chiave] ?? chiave;
}
