//! Persistenza delle collection su file.
//!
//! Struttura su disco (la root del workspace è anche un repo git):
//! ```text
//! workspace/
//!   User APIs/            <- una collezione = una cartella di primo livello
//!     login.json          <- una richiesta = un file JSON formattato
//!     auth/               <- sottocartella (raggruppamento)
//!       refresh.json
//!   environments/         <- riservata agli ambienti (non è una collezione)
//! ```
//! Il JSON è "pretty" (indentato) così i diff di Git restano puliti.

use crate::model::{
    Albero, Auth, Catena, CatenaSuDisco, Collezione, ConfigCartella, Environment,
    EnvironmentSuDisco, EsportaCollezione, Header, Nodo, NodoExport, Richiesta, RisultatoImport,
    RisultatoTest, RunSummary, Variabile, VoceStoria,
};
use crate::snapshot;
use crate::har;
use crate::openapi;
use crate::postman::{self, ImportPostman};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

/// Nome della cartella riservata agli ambienti (non è una collezione).
const DIR_ENV: &str = "environments";
/// Nome della cartella riservata alle catene di run (non è una collezione).
const DIR_RUNS: &str = "runs";
/// File (gitignorato) dove finiscono i valori delle variabili segrete.
const FILE_SECRETS: &str = ".rustman-secrets.json";
/// File (gitignorato) con la cronologia delle richieste inviate.
const FILE_HISTORY: &str = ".rustman-history.json";
/// Numero massimo di voci di cronologia conservate.
const MAX_STORIA: usize = 200;
/// File di configurazione ereditabile dentro una cartella/collezione.
const FILE_CARTELLA: &str = "_rustman.json";

/// Archivio dei segreti: file dell'ambiente → (chiave variabile → valore).
type ArchivioSegreti = HashMap<String, HashMap<String, String>>;

/// Trasforma un nome leggibile in un nome file sicuro (es. "Update User" -> "update-user").
pub fn slug(nome: &str) -> String {
    let mut s = String::new();
    let mut trattino = false;
    for c in nome.trim().chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c.to_ascii_lowercase());
            trattino = false;
        } else if !trattino && !s.is_empty() {
            s.push('-');
            trattino = true;
        }
    }
    while s.ends_with('-') {
        s.pop();
    }
    if s.is_empty() {
        s.push_str("senza-nome");
    }
    s
}

/// Carica l'albero completo: ogni cartella di primo livello è una collezione.
pub fn carica_albero(root: &Path) -> io::Result<Albero> {
    let mut albero: Albero = Vec::new();
    if !root.exists() {
        return Ok(albero);
    }

    let mut cartelle: Vec<_> = fs::read_dir(root)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter(|e| !nome_nascosto(&e.file_name().to_string_lossy()))
        .collect();
    cartelle.sort_by_key(|e| e.file_name());

    for cartella in cartelle {
        let nome = cartella.file_name().to_string_lossy().to_string();
        let figli = leggi_cartella(root, &nome)?;
        albero.push(Collezione {
            nome: nome.clone(),
            dir: nome,
            figli,
        });
    }
    Ok(albero)
}

/// Legge ricorsivamente il contenuto di una cartella (relativa alla root):
/// prima le sottocartelle, poi le richieste (file .json).
fn leggi_cartella(root: &Path, rel: &str) -> io::Result<Vec<Nodo>> {
    let abs = root.join(rel);
    let mut entries: Vec<_> = fs::read_dir(&abs)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());

    let mut figli = Vec::new();

    // Sottocartelle (ricorsione).
    for e in &entries {
        let nome = e.file_name().to_string_lossy().to_string();
        if e.path().is_dir() && !nome_nascosto(&nome) {
            let sub_rel = format!("{}/{}", rel, nome);
            let sub = leggi_cartella(root, &sub_rel)?;
            figli.push(Nodo::Cartella {
                nome,
                dir: sub_rel,
                figli: sub,
            });
        }
    }
    // Richieste (file .json validi).
    for e in &entries {
        let p = e.path();
        if p.extension().map(|x| x == "json").unwrap_or(false) {
            let testo = fs::read_to_string(&p)?;
            if let Ok(richiesta) = serde_json::from_str::<Richiesta>(&testo) {
                let file = format!("{}/{}", rel, e.file_name().to_string_lossy());
                figli.push(Nodo::Richiesta {
                    nome: richiesta.nome.clone(),
                    file,
                    richiesta,
                });
            }
        }
    }
    Ok(figli)
}

/// Salva una richiesta in una cartella (anche annidata) e ne restituisce il percorso.
/// Se `file_precedente` ha un nome diverso (rinomina), il vecchio file viene rimosso.
pub fn salva_richiesta(
    root: &Path,
    dir: &str,
    file_precedente: Option<&str>,
    richiesta: &Richiesta,
) -> io::Result<String> {
    fs::create_dir_all(root.join(dir))?;
    let rel = format!("{}/{}.json", dir, slug(&richiesta.nome));

    let testo = serde_json::to_string_pretty(richiesta)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(root.join(&rel), testo)?;

    if let Some(prec) = file_precedente {
        if prec != rel {
            let _ = fs::remove_file(root.join(prec));
        }
    }
    Ok(rel)
}

/// Crea una cartella (collezione se `dir_genitore` è vuoto) e ne restituisce il percorso.
pub fn crea_cartella(root: &Path, dir_genitore: &str, nome: &str) -> io::Result<String> {
    let base = slug(nome);
    let rel = if dir_genitore.is_empty() {
        base
    } else {
        format!("{}/{}", dir_genitore, base)
    };
    fs::create_dir_all(root.join(&rel))?;
    Ok(rel)
}

/// Crea una collezione (cartella di primo livello).
pub fn crea_collezione(root: &Path, nome: &str) -> io::Result<String> {
    crea_cartella(root, "", nome)
}

/// Crea una nuova richiesta vuota (GET) dentro una cartella.
pub fn crea_richiesta(root: &Path, dir: &str, nome: &str) -> io::Result<String> {
    let richiesta = Richiesta {
        nome: nome.to_string(),
        metodo: "GET".to_string(),
        url: "https://".to_string(),
        headers: Vec::new(),
        params: Vec::new(),
        auth: Auth::default(),
        body: String::new(),
        body_mode: "raw".to_string(),
        form: Vec::new(),
        tests: Vec::new(),
        pre_script: String::new(),
        post_script: String::new(),
        impostazioni: Default::default(),
    };
    salva_richiesta(root, dir, None, &richiesta)
}

/// Elimina un file (richiesta) dato il suo percorso relativo alla root.
pub fn elimina(root: &Path, file_relativo: &str) -> io::Result<()> {
    fs::remove_file(root.join(file_relativo))
}

/// Rinomina una cartella (mantenendo il genitore) e restituisce il nuovo percorso.
pub fn rinomina_cartella(root: &Path, dir: &str, nuovo_nome: &str) -> io::Result<String> {
    let genitore = Path::new(dir)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .filter(|s| !s.is_empty());
    let base = slug(nuovo_nome);
    let nuova = match genitore {
        Some(g) => format!("{}/{}", g, base),
        None => base,
    };
    if nuova != dir {
        fs::rename(root.join(dir), root.join(&nuova))?;
    }
    Ok(nuova)
}

/// Elimina un'intera cartella (e contenuto).
pub fn elimina_cartella(root: &Path, dir: &str) -> io::Result<()> {
    fs::remove_dir_all(root.join(dir))
}

/// True se la cartella va ignorata (nascosta o riservata).
fn nome_nascosto(nome: &str) -> bool {
    nome.starts_with('.') || nome == DIR_ENV || nome == DIR_RUNS
}

// ===================== Environments / variabili ==============================

/// Carica tutti gli ambienti dalla cartella `environments/`.
pub fn carica_environments(root: &Path) -> io::Result<Vec<EnvironmentSuDisco>> {
    let dir = root.join(DIR_ENV);
    let mut lista = Vec::new();
    if !dir.exists() {
        return Ok(lista);
    }
    let mut files: Vec<_> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
        .collect();
    files.sort_by_key(|e| e.file_name());

    let segreti = carica_segreti(root);
    for f in files {
        let testo = fs::read_to_string(f.path())?;
        if let Ok(mut environment) = serde_json::from_str::<Environment>(&testo) {
            let rel = format!("{}/{}", DIR_ENV, f.file_name().to_string_lossy());
            // Reintegra i valori segreti dall'archivio gitignorato.
            if let Some(s) = segreti.get(&rel) {
                for v in environment.variabili.iter_mut() {
                    if v.segreto {
                        if let Some(val) = s.get(&v.chiave) {
                            v.valore = val.clone();
                        }
                    }
                }
            }
            lista.push(EnvironmentSuDisco { file: rel, environment });
        }
    }
    Ok(lista)
}

/// Salva un ambiente e restituisce il suo percorso relativo (rinominando se serve).
/// I valori delle variabili `segreto` non finiscono nel file committato in git,
/// ma in `.rustman-secrets.json` (gitignorato).
pub fn salva_environment(
    root: &Path,
    file_precedente: Option<&str>,
    env: &Environment,
) -> io::Result<String> {
    let dir = root.join(DIR_ENV);
    fs::create_dir_all(&dir)?;
    let rel = format!("{}/{}.json", DIR_ENV, slug(&env.nome));

    // Separa i segreti: nel file su disco il loro valore resta vuoto.
    let mut segreti_env: HashMap<String, String> = HashMap::new();
    let variabili: Vec<Variabile> = env
        .variabili
        .iter()
        .map(|v| {
            if v.segreto {
                if !v.valore.is_empty() {
                    segreti_env.insert(v.chiave.clone(), v.valore.clone());
                }
                Variabile {
                    chiave: v.chiave.clone(),
                    valore: String::new(),
                    segreto: true,
                }
            } else {
                v.clone()
            }
        })
        .collect();
    let env_pubblico = Environment {
        nome: env.nome.clone(),
        variabili,
    };

    let testo = serde_json::to_string_pretty(&env_pubblico)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(root.join(&rel), testo)?;

    // Aggiorna l'archivio dei segreti (gestendo l'eventuale rinomina).
    let mut segreti = carica_segreti(root);
    if let Some(prec) = file_precedente {
        if prec != rel {
            let _ = fs::remove_file(root.join(prec));
            segreti.remove(prec);
        }
    }
    if segreti_env.is_empty() {
        segreti.remove(&rel);
    } else {
        segreti.insert(rel.clone(), segreti_env);
        assicura_gitignore(root, FILE_SECRETS)?;
    }
    salva_segreti(root, &segreti)?;

    Ok(rel)
}

/// Elimina un ambiente dato il suo percorso relativo (e i suoi segreti).
pub fn elimina_environment(root: &Path, file_relativo: &str) -> io::Result<()> {
    let mut segreti = carica_segreti(root);
    if segreti.remove(file_relativo).is_some() {
        salva_segreti(root, &segreti)?;
    }
    fs::remove_file(root.join(file_relativo))
}

/// Carica l'archivio dei segreti (vuoto se il file non c'è o è illeggibile).
fn carica_segreti(root: &Path) -> ArchivioSegreti {
    fs::read_to_string(root.join(FILE_SECRETS))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Scrive l'archivio dei segreti; se vuoto, rimuove del tutto il file.
fn salva_segreti(root: &Path, arch: &ArchivioSegreti) -> io::Result<()> {
    let p = root.join(FILE_SECRETS);
    if arch.is_empty() {
        let _ = fs::remove_file(&p);
        return Ok(());
    }
    let testo = serde_json::to_string_pretty(arch)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(&p, testo)
}

/// Si assicura che `nome` sia presente nel `.gitignore` del workspace.
fn assicura_gitignore(root: &Path, nome: &str) -> io::Result<()> {
    let p = root.join(".gitignore");
    let attuale = fs::read_to_string(&p).unwrap_or_default();
    if attuale.lines().any(|l| l.trim() == nome) {
        return Ok(());
    }
    let mut nuovo = attuale;
    if !nuovo.is_empty() && !nuovo.ends_with('\n') {
        nuovo.push('\n');
    }
    nuovo.push_str(nome);
    nuovo.push('\n');
    fs::write(&p, nuovo)
}

// ===================== History / replay ======================================

/// Carica la cronologia delle richieste (la più recente per prima).
pub fn carica_storia(root: &Path) -> Vec<VoceStoria> {
    fs::read_to_string(root.join(FILE_HISTORY))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Aggiunge una voce in cima alla cronologia (troncata a `MAX_STORIA`).
pub fn aggiungi_storia(root: &Path, voce: VoceStoria) -> io::Result<()> {
    let mut storia = carica_storia(root);
    storia.insert(0, voce);
    storia.truncate(MAX_STORIA);
    assicura_gitignore(root, FILE_HISTORY)?;
    let testo = serde_json::to_string_pretty(&storia)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(root.join(FILE_HISTORY), testo)
}

/// Svuota la cronologia (rimuove il file).
pub fn pulisci_storia(root: &Path) -> io::Result<()> {
    let p = root.join(FILE_HISTORY);
    if p.exists() {
        fs::remove_file(p)?;
    }
    Ok(())
}

// ===================== Diff fra due collezioni ===============================

/// Confronta due collezioni esportate (formato `EsportaCollezione` JSON) e
/// segnala le richieste aggiunte, rimosse o modificate. `None` se il JSON non è
/// valido. Le richieste sono identificate dal percorso "cartella/.../nome".
pub fn diff_collezioni(a_json: &str, b_json: &str) -> Option<crate::model::DriftReport> {
    let a: EsportaCollezione = serde_json::from_str(a_json).ok()?;
    let b: EsportaCollezione = serde_json::from_str(b_json).ok()?;
    let ma = appiattisci_export(&a);
    let mb = appiattisci_export(&b);

    let mut report = crate::model::DriftReport::default();
    for (k, sig_b) in &mb {
        match ma.get(k) {
            None => report.aggiunti.push(k.clone()),
            Some(sig_a) if sig_a != sig_b => report.modificati.push(k.clone()),
            _ => {}
        }
    }
    for k in ma.keys() {
        if !mb.contains_key(k) {
            report.rimossi.push(k.clone());
        }
    }
    report.aggiunti.sort();
    report.rimossi.sort();
    report.modificati.sort();
    Some(report)
}

/// Mappa "cartella/.../nome" → firma (metodo + url + body) delle richieste.
fn appiattisci_export(e: &EsportaCollezione) -> HashMap<String, String> {
    let mut m = HashMap::new();
    raccogli_export_flat(&e.figli, "", &mut m);
    m
}

fn raccogli_export_flat(figli: &[NodoExport], prefisso: &str, m: &mut HashMap<String, String>) {
    for n in figli {
        match n {
            NodoExport::Cartella { nome, figli } => {
                raccogli_export_flat(figli, &format!("{prefisso}{nome}/"), m);
            }
            NodoExport::Richiesta { richiesta } => {
                let key = format!("{prefisso}{}", richiesta.nome);
                let sig = format!("{} {} {}", richiesta.metodo, richiesta.url, richiesta.body);
                m.insert(key, sig);
            }
        }
    }
}

// ===================== Find & Replace ========================================

/// Cerca e sostituisce un testo nei campi delle richieste di tutte le collezioni
/// (url, body, chiavi/valori di header e params, token/utente auth). Salva i file
/// modificati e restituisce quante richieste sono state toccate.
pub fn trova_sostituisci(root: &Path, cerca: &str, con: &str) -> io::Result<usize> {
    if cerca.is_empty() {
        return Ok(0);
    }
    let albero = carica_albero(root)?;
    let mut totale = 0;
    for coll in &albero {
        totale += sostituisci_in_nodi(root, &coll.figli, cerca, con)?;
    }
    Ok(totale)
}

fn sostituisci_in_nodi(root: &Path, figli: &[Nodo], cerca: &str, con: &str) -> io::Result<usize> {
    let mut n = 0;
    for nodo in figli {
        match nodo {
            Nodo::Cartella { figli, .. } => n += sostituisci_in_nodi(root, figli, cerca, con)?,
            Nodo::Richiesta { file, richiesta, .. } => {
                let mut r = richiesta.clone();
                let mut cambiato = false;
                let mut applica = |s: &mut String| {
                    if s.contains(cerca) {
                        *s = s.replace(cerca, con);
                        cambiato = true;
                    }
                };
                applica(&mut r.url);
                applica(&mut r.body);
                applica(&mut r.auth.token);
                applica(&mut r.auth.utente);
                for h in r.headers.iter_mut().chain(r.params.iter_mut()) {
                    applica(&mut h.chiave);
                    applica(&mut h.valore);
                }
                if cambiato {
                    let dir = file.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
                    salva_richiesta(root, dir, Some(file), &r)?;
                    n += 1;
                }
            }
        }
    }
    Ok(n)
}

// ===================== Trend storico dei test ================================

/// File (gitignorato) con lo storico dei run, per il trend del pass-rate.
const FILE_RUNS: &str = ".rustman-runs.json";
const MAX_RUNS: usize = 100;

/// Carica lo storico dei run (il più recente per primo).
pub fn carica_runs(root: &Path) -> Vec<RunSummary> {
    fs::read_to_string(root.join(FILE_RUNS))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Registra un run in cima allo storico (troncato a `MAX_RUNS`).
pub fn registra_run(root: &Path, run: RunSummary) -> io::Result<()> {
    let mut storia = carica_runs(root);
    storia.insert(0, run);
    storia.truncate(MAX_RUNS);
    assicura_gitignore(root, FILE_RUNS)?;
    let testo = serde_json::to_string_pretty(&storia)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(root.join(FILE_RUNS), testo)
}

/// Svuota lo storico dei run.
pub fn pulisci_runs(root: &Path) -> io::Result<()> {
    let p = root.join(FILE_RUNS);
    if p.exists() {
        fs::remove_file(p)?;
    }
    Ok(())
}

// ===================== Snapshot / golden testing =============================

/// Cartella dove vivono le baseline degli snapshot (committate in git).
const DIR_SNAP: &str = ".rustman-snapshots";

fn chiave_snap(file: &str) -> String {
    file.replace('/', "__")
}

/// Carica la baseline dello snapshot per una richiesta (None se non registrata).
pub fn carica_snapshot(root: &Path, file: &str) -> Option<String> {
    fs::read_to_string(root.join(DIR_SNAP).join(chiave_snap(file))).ok()
}

/// Registra/aggiorna la baseline dello snapshot di una richiesta.
pub fn salva_snapshot(root: &Path, file: &str, body: &str) -> io::Result<()> {
    let dir = root.join(DIR_SNAP);
    fs::create_dir_all(&dir)?;
    fs::write(dir.join(chiave_snap(file)), body)
}

/// Valuta lo snapshot: se non c'è baseline la registra (e passa); altrimenti
/// confronta il body con la baseline ignorando gli `ignora` indicati.
pub fn valuta_snapshot(
    root: &Path,
    file: &str,
    ignora: &[String],
    body: &str,
) -> io::Result<RisultatoTest> {
    match carica_snapshot(root, file) {
        None => {
            salva_snapshot(root, file, body)?;
            Ok(RisultatoTest {
                descrizione: "snapshot".into(),
                passato: true,
                dettaglio: "baseline registrata".into(),
            })
        }
        Some(base) => {
            let (ok, det) = snapshot::confronta(&base, body, ignora);
            Ok(RisultatoTest { descrizione: "snapshot".into(), passato: ok, dettaglio: det })
        }
    }
}

// ===================== Ereditarietà di cartella ==============================

/// Carica la configurazione ereditabile di una cartella (vuota se assente).
pub fn carica_config_cartella(root: &Path, dir: &str) -> ConfigCartella {
    fs::read_to_string(root.join(dir).join(FILE_CARTELLA))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Salva la configurazione ereditabile di una cartella.
pub fn salva_config_cartella(root: &Path, dir: &str, cfg: &ConfigCartella) -> io::Result<()> {
    fs::create_dir_all(root.join(dir))?;
    let testo = serde_json::to_string_pretty(cfg)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(root.join(dir).join(FILE_CARTELLA), testo)
}

/// Applica alla richiesta gli header e l'auth ereditati dalle cartelle antenate
/// (`dir` è la cartella che contiene la richiesta). Gli header propri della
/// richiesta hanno la precedenza; l'auth ereditata si usa solo se quella della
/// richiesta è "none". Le cartelle più interne vincono su quelle più esterne.
pub fn eredita(root: &Path, dir: &str, richiesta: &Richiesta) -> Richiesta {
    // Header ereditati: mappa chiave(lowercase) → header, dall'esterno all'interno.
    let mut ered: Vec<(String, Header)> = Vec::new();
    let mut auth_ered: Option<Auth> = None;

    let mut prefisso = String::new();
    for parte in dir.split('/').filter(|s| !s.is_empty()) {
        if !prefisso.is_empty() {
            prefisso.push('/');
        }
        prefisso.push_str(parte);
        let cfg = carica_config_cartella(root, &prefisso);
        for h in cfg.headers {
            if h.chiave.is_empty() {
                continue;
            }
            let k = h.chiave.to_lowercase();
            ered.retain(|(ek, _)| ek != &k); // l'interno sovrascrive l'esterno
            ered.push((k, h));
        }
        if cfg.auth.tipo != "none" {
            auth_ered = Some(cfg.auth);
        }
    }

    let mut r = richiesta.clone();
    // Header propri della richiesta: vincono per chiave.
    let chiavi_req: std::collections::HashSet<String> =
        r.headers.iter().map(|h| h.chiave.to_lowercase()).collect();
    let da_aggiungere: Vec<Header> = ered
        .into_iter()
        .filter(|(k, _)| !chiavi_req.contains(k))
        .map(|(_, h)| h)
        .collect();
    r.headers.extend(da_aggiungere);

    if r.auth.tipo == "none" {
        if let Some(a) = auth_ered {
            r.auth = a;
        }
    }
    r
}

// ===================== Run / catene di chiamate ==============================

/// Carica tutte le catene dalla cartella `runs/`.
pub fn carica_catene(root: &Path) -> io::Result<Vec<CatenaSuDisco>> {
    let dir = root.join(DIR_RUNS);
    let mut lista = Vec::new();
    if !dir.exists() {
        return Ok(lista);
    }
    let mut files: Vec<_> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
        .collect();
    files.sort_by_key(|e| e.file_name());

    for f in files {
        let testo = fs::read_to_string(f.path())?;
        if let Ok(catena) = serde_json::from_str::<Catena>(&testo) {
            let rel = format!("{}/{}", DIR_RUNS, f.file_name().to_string_lossy());
            lista.push(CatenaSuDisco { file: rel, catena });
        }
    }
    Ok(lista)
}

/// Salva una catena e restituisce il suo percorso relativo (rinominando se serve).
pub fn salva_catena(
    root: &Path,
    file_precedente: Option<&str>,
    catena: &Catena,
) -> io::Result<String> {
    let dir = root.join(DIR_RUNS);
    fs::create_dir_all(&dir)?;
    let rel = format!("{}/{}.json", DIR_RUNS, slug(&catena.nome));
    let testo = serde_json::to_string_pretty(catena)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(root.join(&rel), testo)?;
    if let Some(prec) = file_precedente {
        if prec != rel {
            let _ = fs::remove_file(root.join(prec));
        }
    }
    Ok(rel)
}

/// Elimina una catena dato il suo percorso relativo.
pub fn elimina_catena(root: &Path, file_relativo: &str) -> io::Result<()> {
    fs::remove_file(root.join(file_relativo))
}

// ===================== Import / Export =======================================

/// Esporta una collezione (con le sue sottocartelle) in una stringa JSON portabile.
pub fn esporta_collezione(root: &Path, dir: &str) -> io::Result<String> {
    let coll = carica_albero(root)?
        .into_iter()
        .find(|c| c.dir == dir)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "collezione non trovata"))?;

    let esporta = EsportaCollezione {
        rustman: 1,
        nome: coll.nome,
        figli: a_export(&coll.figli),
    };
    serde_json::to_string_pretty(&esporta)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Converte i nodi dell'albero nel formato di esportazione (senza percorsi).
fn a_export(nodi: &[Nodo]) -> Vec<NodoExport> {
    nodi
        .iter()
        .map(|n| match n {
            Nodo::Cartella { nome, figli, .. } => NodoExport::Cartella {
                nome: nome.clone(),
                figli: a_export(figli),
            },
            Nodo::Richiesta { richiesta, .. } => NodoExport::Richiesta {
                richiesta: richiesta.clone(),
            },
        })
        .collect()
}

/// Import "intelligente": riconosce OpenAPI/Swagger, il formato Postman
/// (collection o environment) e il formato nativo Rustman, e salva di conseguenza.
pub fn importa(root: &Path, contenuto: &str) -> io::Result<RisultatoImport> {
    // 1) OpenAPI/Swagger (gestisce anche YAML).
    if let Some((esporta, env)) = openapi::riconosci(contenuto) {
        return salva_collezione_con_env(root, &esporta, env);
    }
    // 1-bis) HAR (export di rete del browser).
    if let Some(esporta) = har::riconosci(contenuto) {
        return salva_collezione_con_env(root, &esporta, None);
    }
    // 2) Postman (collection o environment).
    match postman::riconosci(contenuto) {
        Some(ImportPostman::Collezione(esporta, env)) => {
            salva_collezione_con_env(root, &esporta, env)
        }
        Some(ImportPostman::Environment(env)) => {
            let file = salva_environment(root, None, &env)?;
            Ok(RisultatoImport::Environment { file })
        }
        // 3) Formato nativo Rustman.
        None => {
            let dir = importa_collezione(root, contenuto)?;
            Ok(RisultatoImport::Collezione {
                dir,
                environment: None,
            })
        }
    }
}

/// Salva una collezione e l'eventuale ambiente derivato (variabili di
/// collezione Postman o base URL OpenAPI).
fn salva_collezione_con_env(
    root: &Path,
    esporta: &EsportaCollezione,
    env: Option<Environment>,
) -> io::Result<RisultatoImport> {
    let dir = importa_export(root, esporta)?;
    let environment = match env {
        Some(e) => Some(salva_environment(root, None, &e)?),
        None => None,
    };
    Ok(RisultatoImport::Collezione { dir, environment })
}

/// Importa una collezione dal formato nativo Rustman; restituisce la dir creata.
pub fn importa_collezione(root: &Path, contenuto: &str) -> io::Result<String> {
    let esporta: EsportaCollezione = serde_json::from_str(contenuto)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    importa_export(root, &esporta)
}

/// Scrive una collezione (già in formato `EsportaCollezione`) su disco,
/// scegliendo un nome di cartella libero; restituisce la dir creata.
fn importa_export(root: &Path, esporta: &EsportaCollezione) -> io::Result<String> {
    let dir = dir_unica(root, &slug(&esporta.nome));
    fs::create_dir_all(root.join(&dir))?;
    scrivi_export(root, &dir, &esporta.figli)?;
    Ok(dir)
}

/// Scrive ricorsivamente i nodi di un'esportazione dentro una cartella.
fn scrivi_export(root: &Path, dir: &str, figli: &[NodoExport]) -> io::Result<()> {
    for n in figli {
        match n {
            NodoExport::Richiesta { richiesta } => {
                salva_richiesta(root, dir, None, richiesta)?;
            }
            NodoExport::Cartella { nome, figli } => {
                let sub = crea_cartella(root, dir, nome)?;
                scrivi_export(root, &sub, figli)?;
            }
        }
    }
    Ok(())
}

/// Trova un nome di cartella libero, aggiungendo -2, -3, ... se già esiste.
fn dir_unica(root: &Path, base: &str) -> String {
    if !root.join(base).exists() {
        return base.to_string();
    }
    let mut i = 2;
    loop {
        let candidato = format!("{}-{}", base, i);
        if !root.join(&candidato).exists() {
            return candidato;
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn richiesta(nome: &str) -> Richiesta {
        Richiesta {
            nome: nome.into(),
            metodo: "GET".into(),
            url: "https://x".into(),
            headers: vec![],
            params: vec![],
            auth: Auth::default(),
            body: String::new(),
            body_mode: "raw".into(),
            form: vec![],
            tests: vec![],
            pre_script: String::new(),
            post_script: String::new(),
            impostazioni: Default::default(),
        }
    }

    /// Conta le richieste in un albero di nodi (ricorsivo).
    fn conta_richieste(figli: &[Nodo]) -> usize {
        figli
            .iter()
            .map(|n| match n {
                Nodo::Richiesta { .. } => 1,
                Nodo::Cartella { figli, .. } => conta_richieste(figli),
            })
            .sum()
    }

    #[test]
    fn slug_pulisce_i_nomi() {
        assert_eq!(slug("Update User"), "update-user");
        assert_eq!(slug("  GET /profile!! "), "get-profile");
        assert_eq!(slug(""), "senza-nome");
    }

    #[test]
    fn cartelle_annidate() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        crea_collezione(root, "Test").unwrap();
        salva_richiesta(root, "test", None, &richiesta("Ping")).unwrap();
        let sub = crea_cartella(root, "test", "Auth").unwrap();
        assert_eq!(sub, "test/auth");
        salva_richiesta(root, "test/auth", None, &richiesta("Login")).unwrap();

        let albero = carica_albero(root).unwrap();
        assert_eq!(albero.len(), 1);
        // 1 richiesta diretta + 1 dentro la sottocartella
        assert_eq!(conta_richieste(&albero[0].figli), 2);
        // c'è una cartella "auth" tra i figli
        let ha_cartella = albero[0]
            .figli
            .iter()
            .any(|n| matches!(n, Nodo::Cartella { dir, .. } if dir == "test/auth"));
        assert!(ha_cartella);
    }

    #[test]
    fn esporta_e_importa_con_sottocartelle() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        crea_collezione(root, "User APIs").unwrap();
        salva_richiesta(root, "user-apis", None, &richiesta("Login")).unwrap();
        crea_cartella(root, "user-apis", "Admin").unwrap();
        salva_richiesta(root, "user-apis/admin", None, &richiesta("Ban")).unwrap();

        let json = esporta_collezione(root, "user-apis").unwrap();
        let nuova_dir = importa_collezione(root, &json).unwrap();
        assert_ne!(nuova_dir, "user-apis");

        let albero = carica_albero(root).unwrap();
        assert_eq!(albero.len(), 2);
        let importata = albero.iter().find(|c| c.dir == nuova_dir).unwrap();
        assert_eq!(conta_richieste(&importata.figli), 2); // Login + Ban
    }

    #[test]
    fn variabili_segrete_fuori_da_git() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let env = Environment {
            nome: "Prod".into(),
            variabili: vec![
                Variabile { chiave: "base_url".into(), valore: "https://x".into(), segreto: false },
                Variabile { chiave: "token".into(), valore: "s3cr3t".into(), segreto: true },
            ],
        };
        let rel = salva_environment(root, None, &env).unwrap();

        // Il file committato NON contiene il valore segreto, ma sì quello pubblico.
        let su_disco = fs::read_to_string(root.join(&rel)).unwrap();
        assert!(!su_disco.contains("s3cr3t"));
        assert!(su_disco.contains("https://x"));
        // .gitignore creato con la riga del file dei segreti.
        let gi = fs::read_to_string(root.join(".gitignore")).unwrap();
        assert!(gi.contains(FILE_SECRETS));

        // Il caricamento reintegra il valore segreto.
        let envs = carica_environments(root).unwrap();
        let prod = envs.iter().find(|e| e.file == rel).unwrap();
        let tok = prod.environment.variabili.iter().find(|v| v.chiave == "token").unwrap();
        assert_eq!(tok.valore, "s3cr3t");
        assert!(tok.segreto);

        // Eliminando l'ambiente spariscono anche i suoi segreti.
        elimina_environment(root, &rel).unwrap();
        assert!(carica_segreti(root).get(&rel).is_none());
    }

    #[test]
    fn diff_di_due_collezioni() {
        let a = r#"{"rustman":1,"nome":"API","figli":[
            {"tipo":"richiesta","richiesta":{"nome":"Login","metodo":"POST","url":"https://x/login","body":""}},
            {"tipo":"richiesta","richiesta":{"nome":"Vecchia","metodo":"GET","url":"https://x/old","body":""}}
        ]}"#;
        let b = r#"{"rustman":1,"nome":"API","figli":[
            {"tipo":"richiesta","richiesta":{"nome":"Login","metodo":"POST","url":"https://x/login/v2","body":""}},
            {"tipo":"richiesta","richiesta":{"nome":"Nuova","metodo":"GET","url":"https://x/new","body":""}}
        ]}"#;
        let d = diff_collezioni(a, b).unwrap();
        assert_eq!(d.aggiunti, vec!["Nuova"]);
        assert_eq!(d.rimossi, vec!["Vecchia"]);
        assert_eq!(d.modificati, vec!["Login"]); // url cambiata
    }

    #[test]
    fn find_replace_su_url() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        crea_collezione(root, "API").unwrap();
        let mut r = richiesta("Get");
        r.url = "https://vecchio.test/v1".into();
        salva_richiesta(root, "api", None, &r).unwrap();

        let n = trova_sostituisci(root, "vecchio.test", "nuovo.test").unwrap();
        assert_eq!(n, 1);
        let albero = carica_albero(root).unwrap();
        let Nodo::Richiesta { richiesta, .. } = &albero[0].figli[0] else { panic!() };
        assert_eq!(richiesta.url, "https://nuovo.test/v1");
    }

    #[test]
    fn ereditarieta_header_e_auth() {
        use crate::model::ConfigCartella;
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        crea_collezione(root, "API").unwrap();
        crea_cartella(root, "api", "Admin").unwrap();
        salva_richiesta(root, "api/admin", None, &richiesta("Ban")).unwrap();

        // Config sulla collezione: header + bearer.
        let cfg = ConfigCartella {
            headers: vec![Header { chiave: "X-App".into(), valore: "rustman".into(), attivo: true }],
            auth: Auth { tipo: "bearer".into(), token: "T".into(), ..Auth::default() },
        };
        salva_config_cartella(root, "api", &cfg).unwrap();

        let albero = carica_albero(root).unwrap();
        let Nodo::Cartella { figli, .. } = &albero[0].figli[0] else { panic!() };
        let Nodo::Richiesta { richiesta, .. } = &figli[0] else { panic!() };

        let r = eredita(root, "api/admin", richiesta);
        assert!(r.headers.iter().any(|h| h.chiave == "X-App" && h.valore == "rustman"));
        assert_eq!(r.auth.tipo, "bearer");
        assert_eq!(r.auth.token, "T");
        // Il file di config non deve comparire come richiesta.
        assert_eq!(figli.len(), 1);
    }

    #[test]
    fn catene_roundtrip() {
        use crate::model::{Catena, Passo};
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        let c = Catena {
            nome: "Flusso Login".into(),
            passi: vec![Passo { file: "test/login.json".into() }],
        };
        let rel = salva_catena(root, None, &c).unwrap();
        assert_eq!(rel, "runs/flusso-login.json");

        let catene = carica_catene(root).unwrap();
        assert_eq!(catene.len(), 1);
        assert_eq!(catene[0].catena.passi.len(), 1);

        // La cartella runs/ non deve comparire tra le collezioni.
        assert!(carica_albero(root).unwrap().is_empty());
    }
}
