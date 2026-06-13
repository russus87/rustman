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
- **Auth** Bearer/Basic e **OAuth 2.0** (client_credentials/password) con "Ottieni token".
- **Pre/Post-script** in JavaScript (API stile Postman `pm.*`), eseguiti anche dalla CLI.
- **History** delle richieste inviate con **replay** e **diff** fra due risposte.
- **Copia come cURL** e import di una richiesta da un comando cURL.
- **Variabili dinamiche**: `{{$timestamp}}`, `{{$isoTimestamp}}`, `{{$randomUUID}}`, `{{$randomInt}}`, `{{$randomFloat}}`, con anteprima dell'URL risolto.
- **Contract testing**: asserzione `schema` (JSON Schema), popolata in automatico dall'import OpenAPI.
- **Command Palette** (Ctrl/Cmd+K) per aprire richieste, ambienti, viste e azioni.
- **Generatore di documentazione** HTML dalle collezioni.
- **Cattura dal response**: dai campi JSON crei variabili o asserzioni con un click.
- **Trend dei tempi** per endpoint nella History (sparkline, avg/p95).
- **Drift detection** fra due spec OpenAPI (endpoint aggiunti/rimossi/modificati).
- **Find & Replace** su tutte le richieste; **ereditarietà** di header/auth per cartella.
- **Import HAR** (export di rete del browser).
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
Esegue le richieste di un workspace, **gli script `pm.*`** e verifica le asserzioni
(campo `tests` + `pm.test`), uscendo con codice ≠ 0 in caso di fallimento. Le
variabili `{{...}}` vengono risolte e il var-chaining fra i passi è supportato.
```bash
cargo run -p rustman-cli -- run <workspace> [--env <nome>] \
  [--collection <dir>] [--chain <nome>] [--data dati.csv|dati.json] [--junit report.xml]
```
Con `--data` esegue un'iterazione per riga del file (run **data-driven**), sostituendo
le variabili con i valori della riga. Con `--retry N --delay S` riprova una richiesta
finché le sue asserzioni passano (poll-until-condition).

## Struttura
- `core/` — logica riutilizzabile (HTTP, storage, git, test, perf, import, doc, diff, script).
- `src-tauri/` — app desktop Tauri.
- `server/` — server web (Axum) che espone `core` via REST.
- `cli/` — esecutore headless (`rustman`) per la CI.
- `src/` — frontend Svelte.
