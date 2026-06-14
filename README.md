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
- **Copia come codice** (cURL, fetch JS, Python requests) e import da un comando cURL.
- **Variabili dinamiche**: `{{$timestamp}}`, `{{$isoTimestamp}}`, `{{$randomUUID}}`, `{{$randomInt}}`, `{{$randomFloat}}`, con anteprima dell'URL risolto.
- **Contract testing**: asserzione `schema` (JSON Schema), popolata in automatico dall'import OpenAPI.
- **Command Palette** (Ctrl/Cmd+K) per aprire richieste, ambienti, viste e azioni.
- **Generatore di documentazione** HTML dalle collezioni.
- **Cattura dal response**: dai campi JSON crei variabili o asserzioni con un click.
- **Trend dei tempi** per endpoint nella History (sparkline, avg/p95).
- **Drift detection** fra due spec OpenAPI (endpoint aggiunti/rimossi/modificati).
- **Find & Replace** su tutte le richieste; **ereditarietà** di header/auth per cartella.
- **Import HAR** (export di rete del browser).
- **Snapshot / golden testing** con ignore-paths e approvazione della baseline.
- **API test coverage** dallo spec OpenAPI; **variabili faker** (`{{$name}}`, `{{$email}}`, …).
- **Test di carico** a N richieste o a **durata/RPS** con warmup; report e grafici.
- **WebSocket / SSE**: console interattiva (invio/ricezione) e stream di eventi live.
- **Impostazioni per-richiesta** (timeout, redirect, verifica TLS, retry su 429),
  **cookie jar** di sessione e **rate-limit aware** (`Retry-After`).
- **Run**: catene di chiamate per gli integration test.
- Import/export delle collezioni, workspace multipli, autosalvataggio.
- **Import da Postman** (Collection v2.x ed Environment): cartelle, richieste,
  header, query, auth, body, script (`pm.*`), variabili di collezione e script ereditati.
- **Import/Export da OpenAPI/Swagger** (3.x e 2.0, JSON o YAML): operazioni
  raggruppate per tag, parametri, corpo d'esempio dallo schema e base URL come ambiente.
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
finché le sue asserzioni passano (poll-until-condition). `--report-html` genera un
report navigabile; `--update-snapshots` aggiorna le baseline degli snapshot.

Sottocomandi per la CI:
```bash
# test di carico con SLO gate (esce ≠ 0 se la soglia è superata)
cargo run -p rustman-cli -- perf <workspace> --request demo/get.json \
  --duration 30 --concurrency 20 --rps 50 --max-p95 200 --max-error 1
# copertura delle API rispetto a uno spec OpenAPI
cargo run -p rustman-cli -- coverage <workspace> --spec openapi.yaml
# mock server: serve le risposte d'esempio dello spec (dev senza backend)
cargo run -p rustman-cli -- mock --spec openapi.yaml --port 8080
```

## Struttura
- `core/` — logica riutilizzabile (HTTP, storage, git, test, perf, import, doc, diff, script).
- `src-tauri/` — app desktop Tauri.
- `server/` — server web (Axum) che espone `core` via REST.
- `cli/` — esecutore headless (`rustman`) per la CI.
- `src/` — frontend Svelte.
