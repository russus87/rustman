//! `rustman` — esecutore headless di collezioni/catene per la CI (stile Newman).
//!
//! Carica un workspace Rustman, applica un ambiente, invia le richieste e
//! verifica le asserzioni (campo `tests`), uscendo con codice ≠ 0 se qualcosa
//! fallisce. Può produrre un report JUnit XML per le pipeline.
//!
//! Uso:
//! ```text
//! rustman run <workspace> [--env <nome>] [--collection <dir>] [--chain <nome>] [--junit <file>]
//! ```
//!
//! Nota: gli script JS pre/post (`pm.*`) NON vengono eseguiti dalla CLI; solo le
//! asserzioni native (`tests`). La sostituzione delle variabili `{{...}}` invece sì.

mod script;

use rustman_core::model::{Nodo, Richiesta, RisultatoRun, RisultatoTest};
use rustman_core::{http, storage, test, vars};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Opzioni della riga di comando.
struct Opzioni {
    workspace: PathBuf,
    env: Option<String>,
    collezione: Option<String>,
    catena: Option<String>,
    junit: Option<String>,
    /// File dati (CSV o JSON) per i run data-driven: una iterazione per riga.
    dati: Option<String>,
    /// Numero di ritentativi se le asserzioni falliscono (poll-until-condition).
    retry: u32,
    /// Attesa fra i ritentativi, in secondi.
    delay: u64,
    /// File di output per il report HTML del run.
    report_html: Option<String>,
    /// Se true, aggiorna le baseline degli snapshot invece di confrontarle.
    update_snapshots: bool,
    /// Soglia minima di pass-rate (%) per uscire con successo (gate CI).
    min_pass_rate: Option<f64>,
    /// Numero di esecuzioni per la flaky detection (0 = disattivata).
    flaky: u32,
    /// Esegui solo le richieste con questo tag (suite).
    tag: Option<String>,
}

/// Esito dell'esecuzione di una singola richiesta.
struct Esito {
    nome: String,
    metodo: String,
    url: String,
    status: u16,
    status_text: String,
    tempo_ms: u128,
    /// `Some` se la richiesta non è partita (errore di rete/file).
    errore: Option<String>,
    risultati: Vec<RisultatoTest>,
}

#[tokio::main]
async fn main() -> ExitCode {
    let argomenti: Vec<String> = std::env::args().skip(1).collect();
    let esito = match argomenti.first().map(String::as_str) {
        Some("run") => analizza(&argomenti).map(EsitoCmd::Run),
        Some("perf") => analizza_perf(&argomenti).map(EsitoCmd::Perf),
        Some("coverage") => analizza_coverage(&argomenti).map(EsitoCmd::Coverage),
        Some("mock") => analizza_mock(&argomenti).map(EsitoCmd::Mock),
        _ => Err("Comando mancante o sconosciuto (usa: run | perf | coverage | mock).".into()),
    };
    let cmd = match esito {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("{msg}\n");
            stampa_uso();
            return ExitCode::from(2);
        }
    };

    let risultato = match cmd {
        EsitoCmd::Run(o) => esegui(o).await,
        EsitoCmd::Perf(o) => esegui_perf_cli(o).await,
        EsitoCmd::Coverage(o) => esegui_coverage_cli(o),
        EsitoCmd::Mock(o) => esegui_mock_cli(o),
    };
    match risultato {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("Errore: {e}");
            ExitCode::FAILURE
        }
    }
}

enum EsitoCmd {
    Run(Opzioni),
    Perf(OpzioniPerfCli),
    Coverage(OpzioniCoverage),
    Mock(OpzioniMock),
}

fn stampa_uso() {
    eprintln!(
        "Uso:\n\
\x20 rustman run <workspace> [--env <nome>] [--collection <dir>] [--chain <nome>] \\\n\
\x20             [--data <file>] [--retry <n>] [--delay <s>] [--junit <f>] [--report-html <f>] \\\n\
\x20             [--update-snapshots] [--min-pass-rate <pct>] [--flaky <n>] [--tag <tag>]\n\
\x20 rustman perf <workspace> --request <file> [--env <nome>] [--n <N> | --duration <s>] \\\n\
\x20             [--concurrency <c>] [--rps <r>] [--warmup <s>] [--profile costante|spike|soak] \\\n\
\x20             [--spike-rps <r>] [--max-p95 <ms>] [--max-error <pct>]\n\
\x20 rustman coverage <workspace> --spec <openapi.yaml|json>\n\
\x20 rustman mock --spec <openapi.yaml|json> [--port <p>]"
    );
}

/// Opzioni del sottocomando `mock`.
struct OpzioniMock {
    spec: String,
    port: u16,
}

fn analizza_mock(args: &[String]) -> Result<OpzioniMock, String> {
    let mut spec = String::new();
    let mut port = 8080;
    let mut i = 1;
    while i < args.len() {
        let flag = args[i].as_str();
        let val = || args.get(i + 1).cloned().ok_or_else(|| format!("Manca il valore per {flag}"));
        match flag {
            "--spec" => spec = val()?,
            "--port" => port = val()?.parse().map_err(|_| "--port richiede un numero")?,
            altro => return Err(format!("Opzione sconosciuta: {altro}")),
        }
        i += 2;
    }
    if spec.is_empty() {
        return Err("Indica lo spec con --spec <file>.".into());
    }
    Ok(OpzioniMock { spec, port })
}

/// Avvia il mock server: serve le risposte d'esempio dello spec OpenAPI.
fn esegui_mock_cli(o: OpzioniMock) -> Result<bool, String> {
    let spec = std::fs::read_to_string(&o.spec).map_err(|e| format!("spec '{}': {e}", o.spec))?;
    let routes = rustman_core::openapi::mock_routes(&spec)
        .ok_or("Lo spec non è un OpenAPI/Swagger valido")?;
    let server = tiny_http::Server::http(("0.0.0.0", o.port))
        .map_err(|e| format!("impossibile aprire la porta {}: {e}", o.port))?;

    println!("Mock server su http://localhost:{} — {} rotte:", o.port, routes.len());
    for r in &routes {
        println!("  {} {}", r.metodo, r.path);
    }
    println!("(Ctrl+C per fermare)");

    for req in server.incoming_requests() {
        let metodo = req.method().as_str().to_uppercase();
        let url = req.url().to_string();
        let path = url.split('?').next().unwrap_or(&url).to_string();

        let trovata = routes
            .iter()
            .find(|r| r.metodo == metodo && match_path(&r.path, &path));
        let (status, body) = match trovata {
            Some(r) => {
                println!("{metodo} {path} → {}", r.status);
                (r.status, r.body.clone())
            }
            None => {
                println!("{metodo} {path} → 404");
                (404, "{\"error\":\"not found\"}".to_string())
            }
        };
        let resp = tiny_http::Response::from_string(body)
            .with_status_code(status)
            .with_header(intestazione("Content-Type", "application/json"))
            .with_header(intestazione("Access-Control-Allow-Origin", "*"));
        let _ = req.respond(resp);
    }
    Ok(true)
}

fn intestazione(k: &str, v: &str) -> tiny_http::Header {
    tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes()).unwrap()
}

/// Confronta un path templato ("/pets/{id}") con un path concreto ("/pets/7").
fn match_path(templato: &str, concreto: &str) -> bool {
    let a: Vec<&str> = templato.trim_matches('/').split('/').collect();
    let b: Vec<&str> = concreto.trim_matches('/').split('/').collect();
    a.len() == b.len()
        && a.iter().zip(&b).all(|(t, c)| t.starts_with('{') || t == c)
}

/// Analizza gli argomenti: serve il sottocomando `run` e il percorso workspace.
fn analizza(args: &[String]) -> Result<Opzioni, String> {
    match args.first().map(String::as_str) {
        Some("run") => {}
        Some("--help") | Some("-h") | None => return Err("Comando mancante.".into()),
        Some(altro) => return Err(format!("Comando sconosciuto: {altro}")),
    }
    let workspace = args.get(1).ok_or("Manca il percorso del workspace.")?.into();

    let mut opz = Opzioni {
        workspace,
        env: None,
        collezione: None,
        catena: None,
        junit: None,
        dati: None,
        retry: 0,
        delay: 2,
        report_html: None,
        update_snapshots: false,
        min_pass_rate: None,
        flaky: 0,
        tag: None,
    };
    let mut i = 2;
    while i < args.len() {
        let flag = args[i].as_str();
        let val = || {
            args.get(i + 1)
                .cloned()
                .ok_or_else(|| format!("Manca il valore per {flag}"))
        };
        match flag {
            "--env" => opz.env = Some(val()?),
            "--collection" => opz.collezione = Some(val()?),
            "--chain" => opz.catena = Some(val()?),
            "--junit" => opz.junit = Some(val()?),
            "--data" => opz.dati = Some(val()?),
            "--retry" => opz.retry = val()?.parse().map_err(|_| "--retry richiede un numero")?,
            "--delay" => opz.delay = val()?.parse().map_err(|_| "--delay richiede un numero (secondi)")?,
            "--report-html" => opz.report_html = Some(val()?),
            "--update-snapshots" => {
                opz.update_snapshots = true;
                i += 1; // flag senza valore
                continue;
            }
            "--min-pass-rate" => opz.min_pass_rate = Some(val()?.parse().map_err(|_| "--min-pass-rate richiede un numero")?),
            "--flaky" => opz.flaky = val()?.parse().map_err(|_| "--flaky richiede un numero")?,
            "--tag" => opz.tag = Some(val()?),
            altro => return Err(format!("Opzione sconosciuta: {altro}")),
        }
        i += 2;
    }
    Ok(opz)
}

/// Opzioni del sottocomando `perf`.
struct OpzioniPerfCli {
    workspace: PathBuf,
    request: String,
    env: Option<String>,
    opz: rustman_core::model::OpzioniPerf,
    max_p95: Option<u128>,
    max_error_pct: Option<f64>,
}

fn analizza_perf(args: &[String]) -> Result<OpzioniPerfCli, String> {
    let workspace = args.get(1).ok_or("Manca il percorso del workspace.")?.into();
    let mut o = OpzioniPerfCli {
        workspace,
        request: String::new(),
        env: None,
        opz: rustman_core::model::OpzioniPerf { concorrenza: 10, n: 100, ..Default::default() },
        max_p95: None,
        max_error_pct: None,
    };
    let mut i = 2;
    while i < args.len() {
        let flag = args[i].as_str();
        let val = || args.get(i + 1).cloned().ok_or_else(|| format!("Manca il valore per {flag}"));
        let num = || -> Result<u64, String> { val()?.parse().map_err(|_| format!("{flag} richiede un numero")) };
        match flag {
            "--request" => o.request = val()?,
            "--env" => o.env = Some(val()?),
            "--n" => o.opz.n = num()? as usize,
            "--concurrency" => o.opz.concorrenza = num()? as usize,
            "--duration" => o.opz.durata_s = num()?,
            "--rps" => o.opz.rps = num()?,
            "--warmup" => o.opz.warmup_s = num()?,
            "--max-p95" => o.max_p95 = Some(num()? as u128),
            "--max-error" => o.max_error_pct = Some(val()?.parse().map_err(|_| "--max-error richiede un numero")?),
            "--profile" => o.opz.profilo = val()?,
            "--spike-rps" => o.opz.spike_rps = num()?,
            altro => return Err(format!("Opzione sconosciuta: {altro}")),
        }
        i += 2;
    }
    if o.request.is_empty() {
        return Err("Indica la richiesta con --request <file>.".into());
    }
    Ok(o)
}

/// Opzioni del sottocomando `coverage`.
struct OpzioniCoverage {
    workspace: PathBuf,
    spec: String,
}

fn analizza_coverage(args: &[String]) -> Result<OpzioniCoverage, String> {
    let workspace = args.get(1).ok_or("Manca il percorso del workspace.")?.into();
    let mut spec = String::new();
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--spec" => spec = args.get(i + 1).cloned().ok_or("Manca il valore per --spec")?,
            altro => return Err(format!("Opzione sconosciuta: {altro}")),
        }
        i += 2;
    }
    if spec.is_empty() {
        return Err("Indica lo spec con --spec <file>.".into());
    }
    Ok(OpzioniCoverage { workspace, spec })
}

/// Esegue le richieste selezionate. Restituisce `true` se è tutto verde.
async fn esegui(opz: Opzioni) -> Result<bool, String> {
    let root = &opz.workspace;
    if !root.is_dir() {
        return Err(format!("Workspace non trovato: {}", root.display()));
    }

    // Variabili di base dall'ambiente scelto (vuote se non specificato/trovato).
    let base = carica_variabili(root, opz.env.as_deref())?;

    // Mappa file → richiesta da tutto l'albero del workspace.
    let albero = storage::carica_albero(root).map_err(|e| e.to_string())?;
    let mut per_file: HashMap<String, Richiesta> = HashMap::new();
    for coll in &albero {
        raccogli(&coll.figli, &mut per_file);
    }

    // Selezione delle richieste da eseguire (in ordine).
    let selezione = seleziona(root, &opz, &per_file)?;
    if selezione.is_empty() {
        return Err("Nessuna richiesta da eseguire con i filtri indicati.".into());
    }

    // Righe dati per i run data-driven: una iterazione ciascuna (default: una sola
    // iterazione "vuota"). Le variabili di una riga si sovrappongono a quelle di base.
    let righe = match &opz.dati {
        Some(f) => carica_dati(f)?,
        None => vec![HashMap::new()],
    };
    let multi = opz.dati.is_some();

    let mut esiti: Vec<Esito> = Vec::new();
    for (idx, riga) in righe.iter().enumerate() {
        if multi {
            println!("\n— Iterazione {}/{} —", idx + 1, righe.len());
        }
        // Variabili dell'iterazione: base + riga; mutabili per il var-chaining.
        let mut variabili = base.clone();
        for (k, v) in riga {
            variabili.insert(k.clone(), v.clone());
        }
        for (file, richiesta) in &selezione {
            // Applica gli header/auth ereditati dalle cartelle antenate.
            let dir = file.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
            let req = storage::eredita(root, dir, richiesta);
            esiti.push(
                esegui_richiesta(root, file, &req, &mut variabili, opz.retry, opz.delay, opz.update_snapshots)
                    .await,
            );
        }
    }

    let mut tutto_ok = riepilogo(&esiti);
    if let Some(path) = &opz.junit {
        std::fs::write(path, junit(&esiti)).map_err(|e| e.to_string())?;
        eprintln!("Report JUnit scritto in {path}");
    }
    if let Some(path) = &opz.report_html {
        let righe_run: Vec<RisultatoRun> = esiti.iter().map(a_risultato_run).collect();
        let html = rustman_core::report::genera_html(&righe_run, "Rustman — Report del run");
        std::fs::write(path, html).map_err(|e| e.to_string())?;
        eprintln!("Report HTML scritto in {path}");
    }

    // Gate del pass-rate: fallisce se sotto la soglia.
    if let Some(soglia) = opz.min_pass_rate {
        let totali: usize = esiti.iter().map(|e| e.risultati.len()).sum();
        let ok: usize = esiti.iter().flat_map(|e| &e.risultati).filter(|r| r.passato).count();
        let pr = if totali > 0 { ok as f64 / totali as f64 * 100.0 } else { 100.0 };
        if pr < soglia {
            println!("✗ pass-rate {pr:.1}% < soglia {soglia}%");
            tutto_ok = false;
        } else {
            println!("✓ pass-rate {pr:.1}% (soglia {soglia}%)");
        }
    }

    // Flaky detection: riesegue ogni richiesta e segnala gli esiti intermittenti.
    if opz.flaky > 0 {
        flaky_detection(root, &selezione, &base, opz.flaky).await;
    }

    Ok(tutto_ok)
}

/// Esegue ogni richiesta `volte` volte (senza script) e segnala quelle che
/// passano in alcune esecuzioni e falliscono in altre (flaky).
async fn flaky_detection(
    root: &Path,
    selezione: &[(String, Richiesta)],
    base: &HashMap<String, String>,
    volte: u32,
) {
    println!("\n— Flaky detection ({volte} esecuzioni) —");
    let mut intermittenti = 0;
    for (file, richiesta) in selezione {
        let dir = file.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        let req = storage::eredita(root, dir, richiesta);
        if req.tests.is_empty() {
            continue;
        }
        let mut esiti = Vec::new();
        for _ in 0..volte {
            esiti.push(prova_una(&req, base).await);
        }
        let pass = esiti.iter().filter(|&&b| b).count();
        if pass != 0 && pass != esiti.len() {
            intermittenti += 1;
            let nome = if req.nome.is_empty() { file } else { &req.nome };
            println!("  ⚠ FLAKY: {nome} ({pass}/{} pass)", esiti.len());
        }
    }
    if intermittenti == 0 {
        println!("  nessun test instabile rilevato");
    }
}

/// Invio "silenzioso" di una richiesta: true se tutte le asserzioni passano.
async fn prova_una(richiesta: &Richiesta, base: &HashMap<String, String>) -> bool {
    let r = vars::risolvi(richiesta, base);
    match http::invia(&r).await {
        Ok(risposta) => test::valuta(&r.tests, &risposta).iter().all(|x| x.passato),
        Err(_) => false,
    }
}

/// Converte un `Esito` nel modello condiviso per il report HTML.
fn a_risultato_run(e: &Esito) -> RisultatoRun {
    RisultatoRun {
        nome: e.nome.clone(),
        metodo: e.metodo.clone(),
        url: e.url.clone(),
        status: e.status,
        status_text: e.status_text.clone(),
        tempo_ms: e.tempo_ms,
        errore: e.errore.clone().unwrap_or_default(),
        tests: e.risultati.clone(),
    }
}

/// Esegue il test di performance su una singola richiesta e applica gli SLO gate.
async fn esegui_perf_cli(o: OpzioniPerfCli) -> Result<bool, String> {
    let root = &o.workspace;
    let variabili = carica_variabili(root, o.env.as_deref())?;
    let albero = storage::carica_albero(root).map_err(|e| e.to_string())?;
    let mut per_file = HashMap::new();
    for c in &albero {
        raccogli(&c.figli, &mut per_file);
    }
    let richiesta = per_file
        .get(&o.request)
        .ok_or_else(|| format!("Richiesta non trovata: {}", o.request))?;
    let req = vars::risolvi(richiesta, &variabili);

    let modo = if o.opz.durata_s > 0 {
        format!("{}s @ {} conc", o.opz.durata_s, o.opz.concorrenza)
    } else {
        format!("{} req @ {} conc", o.opz.n, o.opz.concorrenza)
    };
    println!("Perf su '{}' ({modo})…", o.request);
    let r = rustman_core::perf::esegui_cfg(&req, &o.opz).await;

    println!(
        "  {} richieste · {} ok · {} errori · {:.1} req/s",
        r.totali, r.ok, r.errori, r.req_al_secondo
    );
    println!(
        "  latenza min {} · media {:.1} · p50 {} · p90 {} · p95 {} · p99 {} ms",
        r.latenza_min, r.latenza_media, r.p50, r.p90, r.p95, r.p99
    );

    let err_pct = if r.totali > 0 { r.errori as f64 / r.totali as f64 * 100.0 } else { 0.0 };
    let mut ok = true;
    if let Some(max) = o.max_p95 {
        if r.p95 > max {
            println!("  ✗ SLO p95: {} ms > soglia {} ms", r.p95, max);
            ok = false;
        }
    }
    if let Some(max) = o.max_error_pct {
        if err_pct > max {
            println!("  ✗ SLO errori: {err_pct:.2}% > soglia {max}%");
            ok = false;
        }
    }
    if ok {
        println!("  ✓ SLO rispettati");
    }
    Ok(ok)
}

/// Calcola e stampa la copertura delle operazioni dello spec OpenAPI.
fn esegui_coverage_cli(o: OpzioniCoverage) -> Result<bool, String> {
    let root = &o.workspace;
    let spec = std::fs::read_to_string(&o.spec).map_err(|e| format!("spec '{}': {e}", o.spec))?;
    let albero = storage::carica_albero(root).map_err(|e| e.to_string())?;
    let cov = rustman_core::openapi::coverage(&spec, &albero)
        .ok_or("Lo spec non è un OpenAPI/Swagger valido")?;

    println!(
        "Copertura: {}/{} operazioni ({:.0}%)",
        cov.coperti, cov.totali, cov.percentuale
    );
    for s in &cov.scoperti {
        println!("  ✗ scoperto: {s}");
    }
    // In CI consideriamo "verde" solo la copertura totale.
    Ok(cov.scoperti.is_empty())
}

/// Costruisce la mappa variabile→valore dall'ambiente con quel nome.
fn carica_variabili(root: &Path, nome: Option<&str>) -> Result<HashMap<String, String>, String> {
    let mut mappa = HashMap::new();
    let Some(nome) = nome else { return Ok(mappa) };
    let envs = storage::carica_environments(root).map_err(|e| e.to_string())?;
    let trovato = envs
        .iter()
        .find(|e| e.environment.nome.eq_ignore_ascii_case(nome))
        .ok_or_else(|| format!("Ambiente '{nome}' non trovato."))?;
    for v in &trovato.environment.variabili {
        mappa.insert(v.chiave.clone(), v.valore.clone());
    }
    Ok(mappa)
}

/// Carica le righe dati da un file CSV o JSON (array di oggetti).
/// Ogni riga è una mappa chiave→valore (i valori non stringa sono convertiti).
fn carica_dati(path: &str) -> Result<Vec<HashMap<String, String>>, String> {
    let testo = std::fs::read_to_string(path).map_err(|e| format!("dati '{path}': {e}"))?;
    if path.ends_with(".json") || testo.trim_start().starts_with('[') {
        let arr: Vec<serde_json::Map<String, serde_json::Value>> =
            serde_json::from_str(&testo).map_err(|e| format!("JSON dati non valido: {e}"))?;
        Ok(arr
            .into_iter()
            .map(|obj| {
                obj.into_iter()
                    .map(|(k, v)| {
                        let s = match v {
                            serde_json::Value::String(s) => s,
                            altro => altro.to_string(),
                        };
                        (k, s)
                    })
                    .collect()
            })
            .collect())
    } else {
        carica_csv(&testo)
    }
}

/// Parsing CSV semplice: prima riga = intestazioni, separatore virgola.
/// (Non gestisce virgole tra virgolette: per casi complessi usare il JSON.)
fn carica_csv(testo: &str) -> Result<Vec<HashMap<String, String>>, String> {
    let mut linee = testo.lines().filter(|l| !l.trim().is_empty());
    let intestazioni: Vec<String> = linee
        .next()
        .ok_or("file CSV vuoto")?
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    Ok(linee
        .map(|riga| {
            let valori: Vec<&str> = riga.split(',').collect();
            intestazioni
                .iter()
                .enumerate()
                .map(|(i, k)| (k.clone(), valori.get(i).map(|v| v.trim().to_string()).unwrap_or_default()))
                .collect()
        })
        .collect())
}

/// Visita ricorsiva dell'albero per raccogliere le richieste per percorso file.
fn raccogli(figli: &[Nodo], out: &mut HashMap<String, Richiesta>) {
    for n in figli {
        match n {
            Nodo::Richiesta { file, richiesta, .. } => {
                out.insert(file.clone(), richiesta.clone());
            }
            Nodo::Cartella { figli, .. } => raccogli(figli, out),
        }
    }
}

/// Decide quali richieste eseguire in base a --chain / --collection / (tutte).
/// Filtra le richieste tenendo solo quelle col tag indicato (se presente).
fn applica_filtro_tag(out: &mut Vec<(String, Richiesta)>, tag: &Option<String>) {
    if let Some(t) = tag {
        out.retain(|(_, r)| r.tags.iter().any(|x| x.eq_ignore_ascii_case(t)));
    }
}

fn seleziona(
    root: &Path,
    opz: &Opzioni,
    per_file: &HashMap<String, Richiesta>,
) -> Result<Vec<(String, Richiesta)>, String> {
    if let Some(nome) = &opz.catena {
        let catene = storage::carica_catene(root).map_err(|e| e.to_string())?;
        let cat = catene
            .iter()
            .find(|c| c.catena.nome.eq_ignore_ascii_case(nome))
            .ok_or_else(|| format!("Catena '{nome}' non trovata."))?;
        let mut out = Vec::new();
        for passo in &cat.catena.passi {
            let r = per_file
                .get(&passo.file)
                .ok_or_else(|| format!("Richiesta della catena non trovata: {}", passo.file))?;
            out.push((passo.file.clone(), r.clone()));
        }
        applica_filtro_tag(&mut out, &opz.tag);
        return Ok(out);
    }

    // Tutte le richieste, opzionalmente filtrate per cartella, in ordine di file.
    let mut out: Vec<(String, Richiesta)> = per_file
        .iter()
        .filter(|(file, _)| match &opz.collezione {
            Some(dir) => file.starts_with(&format!("{dir}/")) || file.starts_with(dir.as_str()),
            None => true,
        })
        .map(|(f, r)| (f.clone(), r.clone()))
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    applica_filtro_tag(&mut out, &opz.tag);
    Ok(out)
}

/// Esegue il pre-script, invia la richiesta (variabili risolte), poi il
/// post-script; raccoglie le asserzioni native e quelle di `pm.test(...)`.
/// Con `retry > 0` riprova finché le asserzioni passano (poll-until-condition).
#[allow(clippy::too_many_arguments)]
async fn esegui_richiesta(
    root: &Path,
    file: &str,
    richiesta: &Richiesta,
    variabili: &mut HashMap<String, String>,
    retry: u32,
    delay: u64,
    update_snapshots: bool,
) -> Esito {
    let nome = if richiesta.nome.is_empty() {
        file.to_string()
    } else {
        richiesta.nome.clone()
    };

    // Pre-script: può impostare variabili usate nella sostituzione.
    if !richiesta.pre_script.is_empty() {
        match script::esegui(&richiesta.pre_script, variabili, richiesta, None) {
            Ok(es) => {
                *variabili = es.variabili;
                stampa_logs(&es.logs);
            }
            Err(e) => eprintln!("  ✗ pre-script: {e}"),
        }
    }

    let tentativi = retry + 1;
    for tentativo in 1..=tentativi {
        let r = vars::risolvi(richiesta, variabili);
        match http::invia(&r).await {
            Ok(risposta) => {
                // Asserzioni native (lo "snapshot" è escluso da test::valuta).
                let mut risultati = if r.tests.is_empty() {
                    Vec::new()
                } else {
                    test::valuta(&r.tests, &risposta)
                };
                // Snapshot / golden testing: confronto con la baseline su file.
                for a in r.tests.iter().filter(|a| a.attivo && a.tipo == "snapshot") {
                    let ignora: Vec<String> = a
                        .atteso
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    let esito = if update_snapshots {
                        storage::salva_snapshot(root, file, &risposta.body)
                            .map(|_| ok_test("snapshot aggiornata"))
                            .unwrap_or_else(|e| ko_test(&e.to_string()))
                    } else {
                        storage::valuta_snapshot(root, file, &ignora, &risposta.body)
                            .unwrap_or_else(|e| ko_test(&e.to_string()))
                    };
                    risultati.push(esito);
                }
                if !richiesta.post_script.is_empty() {
                    match script::esegui(&richiesta.post_script, variabili, &r, Some(&risposta)) {
                        Ok(es) => {
                            *variabili = es.variabili;
                            stampa_logs(&es.logs);
                            risultati.extend(es.tests);
                        }
                        Err(e) => eprintln!("  ✗ post-script: {e}"),
                    }
                }
                let ok = risultati.iter().all(|x| x.passato);
                if ok || tentativo == tentativi {
                    stampa_esito(&nome, &r, Some(&risposta), &risultati, None);
                    return Esito {
                        nome,
                        metodo: r.metodo.clone(),
                        url: r.url.clone(),
                        status: risposta.status,
                        status_text: risposta.status_text.clone(),
                        tempo_ms: risposta.tempo_ms,
                        errore: None,
                        risultati,
                    };
                }
                eprintln!("  … tentativo {tentativo}/{tentativi} non passato, riprovo tra {delay}s");
            }
            Err(e) => {
                if tentativo == tentativi {
                    let msg = e.to_string();
                    stampa_esito(&nome, &r, None, &[], Some(&msg));
                    return Esito {
                        nome,
                        metodo: r.metodo.clone(),
                        url: r.url.clone(),
                        status: 0,
                        status_text: String::new(),
                        tempo_ms: 0,
                        errore: Some(msg),
                        risultati: Vec::new(),
                    };
                }
                eprintln!("  … invio fallito ({e}), riprovo tra {delay}s");
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
    }
    unreachable!("il loop ritorna sempre all'ultimo tentativo")
}

fn ok_test(det: &str) -> RisultatoTest {
    RisultatoTest { descrizione: "snapshot".into(), passato: true, dettaglio: det.into() }
}
fn ko_test(det: &str) -> RisultatoTest {
    RisultatoTest { descrizione: "snapshot".into(), passato: false, dettaglio: det.into() }
}

fn stampa_logs(logs: &[String]) {
    for l in logs {
        println!("  · {l}");
    }
}

fn stampa_esito(
    nome: &str,
    r: &Richiesta,
    risposta: Option<&rustman_core::model::Risposta>,
    risultati: &[RisultatoTest],
    errore: Option<&str>,
) {
    println!("▶ {nome}  {} {}", r.metodo, r.url);
    match (risposta, errore) {
        (Some(resp), _) => println!("  {} {} · {}ms", resp.status, resp.status_text, resp.tempo_ms),
        (_, Some(msg)) => println!("  ✗ richiesta fallita: {msg}"),
        _ => {}
    }
    for r in risultati {
        let segno = if r.passato { "✓" } else { "✗" };
        let dett = if r.dettaglio.is_empty() {
            String::new()
        } else {
            format!(" — {}", r.dettaglio)
        };
        println!("  {segno} {}{dett}", r.descrizione);
    }
}

/// Stampa il riepilogo e restituisce `true` se non c'è alcun fallimento.
fn riepilogo(esiti: &[Esito]) -> bool {
    let mut ok = 0;
    let mut falliti = 0;
    let mut errori = 0;
    for e in esiti {
        if e.errore.is_some() {
            errori += 1;
        }
        for r in &e.risultati {
            if r.passato {
                ok += 1;
            } else {
                falliti += 1;
            }
        }
    }
    println!(
        "\n{} richieste · {ok} test ok · {falliti} falliti · {errori} errori di invio",
        esiti.len()
    );
    falliti == 0 && errori == 0
}

/// Genera un report JUnit XML: un testcase per asserzione (più uno per gli errori di invio).
fn junit(esiti: &[Esito]) -> String {
    let mut casi = String::new();
    let mut totali = 0;
    let mut falliti = 0;
    for e in esiti {
        if let Some(msg) = &e.errore {
            totali += 1;
            falliti += 1;
            casi.push_str(&format!(
                "    <testcase classname=\"{}\" name=\"invio\"><failure message=\"{}\"/></testcase>\n",
                esc(&e.nome),
                esc(msg)
            ));
        }
        for r in &e.risultati {
            totali += 1;
            if r.passato {
                casi.push_str(&format!(
                    "    <testcase classname=\"{}\" name=\"{}\"/>\n",
                    esc(&e.nome),
                    esc(&r.descrizione)
                ));
            } else {
                falliti += 1;
                casi.push_str(&format!(
                    "    <testcase classname=\"{}\" name=\"{}\"><failure message=\"{}\"/></testcase>\n",
                    esc(&e.nome),
                    esc(&r.descrizione),
                    esc(&r.dettaglio)
                ));
            }
        }
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuites>\n  <testsuite name=\"rustman\" tests=\"{totali}\" failures=\"{falliti}\">\n{casi}  </testsuite>\n</testsuites>\n"
    )
}

/// Escape minimale per gli attributi XML.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
