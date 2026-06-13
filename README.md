# Rustman

Client API desktop (e web) ispirato a Postman, scritto in **Rust + Tauri + Svelte**.

## Funzionalità
- Collezioni di richieste con **cartelle annidate**, salvate come file JSON.
- **Git** integrato sul workspace (commit, diff, cronologia, remote con pull/push).
- **Test/asserzioni** sulle risposte e **performance test** con grafici.
- **Environments** e variabili `{{...}}`, query params, auth (Bearer/Basic).
- **Pre/Post-script** in JavaScript (API stile Postman `pm.*`).
- **Run**: catene di chiamate per gli integration test.
- Import/export delle collezioni, workspace multipli, autosalvataggio.
- **Import da Postman** (Collection v2.x ed Environment): cartelle, richieste,
  header, query, auth, body, script (`pm.*`), variabili di collezione e script ereditati.

## Sviluppo
```bash
npm install
cargo tauri dev          # app desktop in sviluppo
```

Versione web:
```bash
npm run build
cargo run -p rustman-server   # http://localhost:1421
```

## Build dei pacchetti
```bash
cargo tauri build        # bundle nella cartella target/release/bundle/
```
Tauri va compilato sul sistema di destinazione. I pacchetti per Linux, Windows e macOS
vengono generati automaticamente da GitHub Actions (`.github/workflows/release.yml`)
quando si pubblica un tag `vX.Y.Z`.

## Struttura
- `core/` — logica riutilizzabile (HTTP, storage, git, test, perf).
- `src-tauri/` — app desktop Tauri.
- `server/` — server web (Axum) che espone `core` via REST.
- `src/` — frontend Svelte.
