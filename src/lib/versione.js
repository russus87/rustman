// Versione dell'app e changelog mostrati nella vista Info.

export const VERSIONE = "0.9.0";

// Dal più recente al più vecchio. Il primo è la versione attuale.
export const CHANGELOG = [
  {
    versione: "0.9.0",
    voci: [
      "Import da Postman: Collection v2.x ed Environment.",
      "Mappa cartelle, richieste, header, query, auth, body e script (pm.*).",
      "Le variabili di collezione diventano un ambiente; gli script di collezione/cartella sono ereditati.",
    ],
  },
  {
    versione: "0.8.0",
    voci: [
      "Pannelli ridimensionabili con il mouse (sidebar, editor/risposta, risposta/performance, log).",
      "Vista Info con versione corrente e changelog.",
      "Pannello Log in basso al centro (richieste, errori, script, run).",
    ],
  },
  {
    versione: "0.7.0",
    voci: [
      "Tab Run: catene di chiamate per gli integration test (stop al primo errore).",
      "Pre-script e Post-script in JavaScript con API stile Postman (pm.*).",
      "Gestione ambienti spostata nel menu laterale.",
    ],
  },
  {
    versione: "0.6.0",
    voci: [
      "Interfaccia stile VS Code: barra attività e viste commutabili.",
      "Richieste in più tab; cartelle annidate nelle collezioni.",
      "Impostazioni con autosalvataggio; Git con remote, pull e push.",
      "Workspace multipli (scelta della cartella).",
    ],
  },
  {
    versione: "0.5.0",
    voci: [
      "Environments e variabili {{...}}; query params e autenticazione (Bearer/Basic).",
      "Rinomina/elimina collezioni; import/export delle collezioni.",
      "Versione web (server) oltre al desktop.",
    ],
  },
  {
    versione: "0.1.0 – 0.4.0",
    voci: [
      "Invio richieste HTTP con metriche; collezioni su file.",
      "Versionamento Git (commit, diff, cronologia).",
      "Test/asserzioni sulle risposte; performance test con grafici.",
    ],
  },
];
