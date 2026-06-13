# Rustman

Client API desktop (e web) ispirato a Postman, scritto in **Rust + Tauri + Svelte**.

## Funzionalità
- Collezioni di richieste con **cartelle annidate**, salvate come file JSON.
- **Git** integrato sul workspace (commit, diff, cronologia, remote con pull/push).
- **Test/asserzioni** sulle risposte e **performance test** con grafici.
- **Environments** e variabili `{{...}}`, query params, auth (Bearer/Basic).
- **Variabili segrete** 🔒: i valori non finiscono nei file committati in git
  (vengono salvati in `.rustman-secrets.json`, gitignorato).
- **Corpo della richiesta** raw, **`x-www-form-urlencoded`** e **`form-data`**
  con upload di file (multipart, su desktop).
- **Pre/Post-script** in JavaScript (API stile Postman `pm.*`).
- **Run**: catene di chiamate per gli integration test.
- Import/export delle collezioni, workspace multipli, autosalvataggio.
- **Import da Postman** (Collection v2.x ed Environment): cartelle, richieste,
  header, query, auth, body, script (`pm.*`), variabili di collezione e script ereditati.
- **Import da OpenAPI/Swagger** (3.x e 2.0, JSON o YAML): operazioni raggruppate
  per tag, parametri, corpo d'esempio dallo schema e base URL come ambiente.
- **CLI headless** (`rustman run`): esegue collezioni/catene in CI con report JUnit.

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

## CLI (esecuzione in CI)
Esegue le richieste di un workspace e verifica le asserzioni (campo `tests`),
uscendo con codice ≠ 0 in caso di fallimento. Gli script JS pre/post non vengono
eseguiti dalla CLI (solo le asserzioni native); le variabili `{{...}}` invece sì.
```bash
cargo run -p rustman-cli -- run <workspace> [--env <nome>] \
  [--collection <dir>] [--chain <nome>] [--junit report.xml]
```

## Struttura
- `core/` — logica riutilizzabile (HTTP, storage, git, test, perf, import Postman/OpenAPI).
- `src-tauri/` — app desktop Tauri.
- `server/` — server web (Axum) che espone `core` via REST.
- `cli/` — esecutore headless (`rustman`) per la CI.
- `src/` — frontend Svelte.
