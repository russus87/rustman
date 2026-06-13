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

use rustman_core::model::{Nodo, Richiesta, RisultatoTest};
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
}

/// Esito dell'esecuzione di una singola richiesta.
struct Esito {
    nome: String,
    /// `Some` se la richiesta non è partita (errore di rete/file).
    errore: Option<String>,
    risultati: Vec<RisultatoTest>,
}

#[tokio::main]
async fn main() -> ExitCode {
    let argomenti: Vec<String> = std::env::args().skip(1).collect();
    let opz = match analizza(&argomenti) {
        Ok(o) => o,
        Err(msg) => {
            eprintln!("{msg}\n");
            stampa_uso();
            return ExitCode::from(2);
        }
    };

    match esegui(opz).await {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("Errore: {e}");
            ExitCode::FAILURE
        }
    }
}

fn stampa_uso() {
    eprintln!(
        "Uso:\n  rustman run <workspace> [--env <nome>] [--collection <dir>] [--chain <nome>] \\\n              [--data <file.csv|file.json>] [--retry <n>] [--delay <s>] [--junit <file>]"
    );
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
            altro => return Err(format!("Opzione sconosciuta: {altro}")),
        }
        i += 2;
    }
    Ok(opz)
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
            esiti.push(esegui_richiesta(file, &req, &mut variabili, opz.retry, opz.delay).await);
        }
    }

    let tutto_ok = riepilogo(&esiti);
    if let Some(path) = &opz.junit {
        std::fs::write(path, junit(&esiti)).map_err(|e| e.to_string())?;
        eprintln!("Report JUnit scritto in {path}");
    }
    Ok(tutto_ok)
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
    Ok(out)
}

/// Esegue il pre-script, invia la richiesta (variabili risolte), poi il
/// post-script; raccoglie le asserzioni native e quelle di `pm.test(...)`.
/// Con `retry > 0` riprova finché le asserzioni passano (poll-until-condition).
async fn esegui_richiesta(
    file: &str,
    richiesta: &Richiesta,
    variabili: &mut HashMap<String, String>,
    retry: u32,
    delay: u64,
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
                let mut risultati = if r.tests.is_empty() {
                    Vec::new()
                } else {
                    test::valuta(&r.tests, &risposta)
                };
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
                    return Esito { nome, errore: None, risultati };
                }
                eprintln!("  … tentativo {tentativo}/{tentativi} non passato, riprovo tra {delay}s");
            }
            Err(e) => {
                if tentativo == tentativi {
                    let msg = e.to_string();
                    stampa_esito(&nome, &r, None, &[], Some(&msg));
                    return Esito { nome, errore: Some(msg), risultati: Vec::new() };
                }
                eprintln!("  … invio fallito ({e}), riprovo tra {delay}s");
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
    }
    // Irraggiungibile: il loop ritorna sempre all'ultimo tentativo.
    Esito { nome, errore: Some("nessun tentativo".into()), risultati: Vec::new() }
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
