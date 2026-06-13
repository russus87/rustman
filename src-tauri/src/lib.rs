//! Punto di ingresso dell'app Tauri: registra i comandi richiamabili dal frontend.
//! La logica vera vive nel crate `rustman_core`; qui facciamo da ponte e gestiamo
//! il percorso del workspace (la cartella, anche repo git, dove vivono le collection).

use rustman_core::{
    git, http,
    model::{
        Albero, Asserzione, Catena, CatenaSuDisco, Commit, Environment, EnvironmentSuDisco,
        FileModificato, Richiesta, RigaDiff, RisultatoPerf, RisultatoTest, Risposta, StatoRepo,
    },
    perf, storage, test, vars,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

/// Configurazione persistente: elenco dei workspace e quello attivo.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
struct Config {
    workspaces: Vec<String>,
    attivo: Option<String>,
}

/// Stato condiviso dell'app che custodisce la `Config`.
struct StatoConfig(Mutex<Config>);

/// Percorso del file di configurazione (in app_config_dir/rustman.json).
fn percorso_config(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("rustman.json"))
}

/// Workspace di default (app_data_dir/workspace).
fn workspace_default(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let base = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(base.join("workspace"))
}

fn carica_config(app: &tauri::AppHandle) -> Config {
    percorso_config(app)
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn salva_config(app: &tauri::AppHandle, cfg: &Config) -> Result<(), String> {
    let p = percorso_config(app)?;
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let testo = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(p, testo).map_err(|e| e.to_string())
}

/// Restituisce (creandola se serve) la cartella del workspace attivo e ne assicura il repo git.
fn workspace(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let attivo = app
        .state::<StatoConfig>()
        .0
        .lock()
        .unwrap()
        .attivo
        .clone();
    let root = match attivo {
        Some(p) => PathBuf::from(p),
        None => workspace_default(app)?,
    };
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    git::assicura_repo(&root).map_err(|e| e.to_string())?;
    Ok(root)
}

// ============================ Comandi HTTP ============================

/// Invia una richiesta HTTP, applicando prima le variabili d'ambiente (se presenti).
#[tauri::command]
async fn invia_richiesta(
    richiesta: Richiesta,
    variabili: Option<HashMap<String, String>>,
) -> Result<Risposta, String> {
    let r = match &variabili {
        Some(v) => vars::risolvi(&richiesta, v),
        None => richiesta,
    };
    http::invia(&r).await.map_err(|e| e.to_string())
}

// ====================== Comandi test e performance ======================

/// Valuta le asserzioni di una richiesta sulla risposta ricevuta.
#[tauri::command]
fn valuta_test(asserzioni: Vec<Asserzione>, risposta: Risposta) -> Vec<RisultatoTest> {
    test::valuta(&asserzioni, &risposta)
}

/// Esegue un test di carico: `n` richieste con `concorrenza` richieste in volo.
#[tauri::command]
async fn esegui_perf(
    richiesta: Richiesta,
    n: usize,
    concorrenza: usize,
    variabili: Option<HashMap<String, String>>,
) -> Result<RisultatoPerf, String> {
    let r = match &variabili {
        Some(v) => vars::risolvi(&richiesta, v),
        None => richiesta,
    };
    Ok(perf::esegui(&r, n, concorrenza).await)
}

// ========================= Comandi workspace =========================

/// Elenco dei workspace e quello attivo.
#[tauri::command]
fn lista_workspaces(app: tauri::AppHandle) -> Config {
    app.state::<StatoConfig>().0.lock().unwrap().clone()
}

/// Aggiunge un workspace all'elenco (senza renderlo attivo).
#[tauri::command]
fn aggiungi_workspace(app: tauri::AppHandle, percorso: String) -> Result<(), String> {
    let cfg = {
        let stato = app.state::<StatoConfig>();
        let mut cfg = stato.0.lock().unwrap();
        if !cfg.workspaces.contains(&percorso) {
            cfg.workspaces.push(percorso);
        }
        cfg.clone()
    };
    salva_config(&app, &cfg)
}

/// Imposta il workspace attivo (aggiungendolo se non presente) e ne assicura il repo.
#[tauri::command]
fn imposta_workspace_attivo(app: tauri::AppHandle, percorso: String) -> Result<(), String> {
    let cfg = {
        let stato = app.state::<StatoConfig>();
        let mut cfg = stato.0.lock().unwrap();
        if !cfg.workspaces.contains(&percorso) {
            cfg.workspaces.push(percorso.clone());
        }
        cfg.attivo = Some(percorso);
        cfg.clone()
    };
    salva_config(&app, &cfg)?;
    workspace(&app)?; // crea cartella + repo se necessario
    Ok(())
}

/// Rimuove un workspace dall'elenco (non cancella i file su disco).
#[tauri::command]
fn rimuovi_workspace(app: tauri::AppHandle, percorso: String) -> Result<(), String> {
    let cfg = {
        let stato = app.state::<StatoConfig>();
        let mut cfg = stato.0.lock().unwrap();
        cfg.workspaces.retain(|p| p != &percorso);
        if cfg.attivo.as_deref() == Some(percorso.as_str()) {
            cfg.attivo = cfg.workspaces.first().cloned();
        }
        cfg.clone()
    };
    salva_config(&app, &cfg)
}

// ========================= Comandi collection =========================

/// Percorso assoluto del workspace (utile da mostrare/ispezionare con git da terminale).
#[tauri::command]
fn percorso_workspace(app: tauri::AppHandle) -> Result<String, String> {
    Ok(workspace(&app)?.to_string_lossy().to_string())
}

/// Carica l'albero completo delle collezioni e richieste.
#[tauri::command]
fn carica_albero(app: tauri::AppHandle) -> Result<Albero, String> {
    let root = workspace(&app)?;
    storage::carica_albero(&root).map_err(|e| e.to_string())
}

/// Salva una richiesta; restituisce il percorso relativo del file scritto.
#[tauri::command]
fn salva_richiesta(
    app: tauri::AppHandle,
    dir: String,
    file_precedente: Option<String>,
    richiesta: Richiesta,
) -> Result<String, String> {
    let root = workspace(&app)?;
    storage::salva_richiesta(&root, &dir, file_precedente.as_deref(), &richiesta)
        .map_err(|e| e.to_string())
}

/// Crea una nuova collezione (cartella di primo livello); restituisce il percorso.
#[tauri::command]
fn crea_collezione(app: tauri::AppHandle, nome: String) -> Result<String, String> {
    let root = workspace(&app)?;
    storage::crea_collezione(&root, &nome).map_err(|e| e.to_string())
}

/// Crea una sottocartella dentro `dir_genitore`; restituisce il percorso.
#[tauri::command]
fn crea_cartella(
    app: tauri::AppHandle,
    dir_genitore: String,
    nome: String,
) -> Result<String, String> {
    let root = workspace(&app)?;
    storage::crea_cartella(&root, &dir_genitore, &nome).map_err(|e| e.to_string())
}

/// Crea una nuova richiesta vuota in una cartella; restituisce il percorso.
#[tauri::command]
fn crea_richiesta(app: tauri::AppHandle, dir: String, nome: String) -> Result<String, String> {
    let root = workspace(&app)?;
    storage::crea_richiesta(&root, &dir, &nome).map_err(|e| e.to_string())
}

/// Elimina una richiesta dato il suo percorso relativo.
#[tauri::command]
fn elimina(app: tauri::AppHandle, file: String) -> Result<(), String> {
    let root = workspace(&app)?;
    storage::elimina(&root, &file).map_err(|e| e.to_string())
}

/// Rinomina una cartella; restituisce il nuovo percorso relativo.
#[tauri::command]
fn rinomina_cartella(
    app: tauri::AppHandle,
    dir: String,
    nuovo_nome: String,
) -> Result<String, String> {
    let root = workspace(&app)?;
    storage::rinomina_cartella(&root, &dir, &nuovo_nome).map_err(|e| e.to_string())
}

/// Elimina un'intera cartella (e contenuto).
#[tauri::command]
fn elimina_cartella(app: tauri::AppHandle, dir: String) -> Result<(), String> {
    let root = workspace(&app)?;
    storage::elimina_cartella(&root, &dir).map_err(|e| e.to_string())
}

/// Esporta una collezione come stringa JSON portabile.
#[tauri::command]
fn esporta_collezione(app: tauri::AppHandle, dir: String) -> Result<String, String> {
    let root = workspace(&app)?;
    storage::esporta_collezione(&root, &dir).map_err(|e| e.to_string())
}

/// Importa una collezione da una stringa JSON; restituisce la dir creata.
#[tauri::command]
fn importa_collezione(app: tauri::AppHandle, contenuto: String) -> Result<String, String> {
    let root = workspace(&app)?;
    storage::importa_collezione(&root, &contenuto).map_err(|e| e.to_string())
}

// ========================= Comandi environments =========================

/// Carica tutti gli ambienti definiti.
#[tauri::command]
fn carica_environments(app: tauri::AppHandle) -> Result<Vec<EnvironmentSuDisco>, String> {
    let root = workspace(&app)?;
    storage::carica_environments(&root).map_err(|e| e.to_string())
}

/// Salva un ambiente; restituisce il percorso relativo del file.
#[tauri::command]
fn salva_environment(
    app: tauri::AppHandle,
    file_precedente: Option<String>,
    environment: Environment,
) -> Result<String, String> {
    let root = workspace(&app)?;
    storage::salva_environment(&root, file_precedente.as_deref(), &environment)
        .map_err(|e| e.to_string())
}

/// Elimina un ambiente dato il suo percorso relativo.
#[tauri::command]
fn elimina_environment(app: tauri::AppHandle, file: String) -> Result<(), String> {
    let root = workspace(&app)?;
    storage::elimina_environment(&root, &file).map_err(|e| e.to_string())
}

// ========================= Comandi catene (Run) =========================

/// Carica tutte le catene di run.
#[tauri::command]
fn carica_catene(app: tauri::AppHandle) -> Result<Vec<CatenaSuDisco>, String> {
    let root = workspace(&app)?;
    storage::carica_catene(&root).map_err(|e| e.to_string())
}

/// Salva una catena; restituisce il percorso relativo del file.
#[tauri::command]
fn salva_catena(
    app: tauri::AppHandle,
    file_precedente: Option<String>,
    catena: Catena,
) -> Result<String, String> {
    let root = workspace(&app)?;
    storage::salva_catena(&root, file_precedente.as_deref(), &catena).map_err(|e| e.to_string())
}

/// Elimina una catena dato il suo percorso relativo.
#[tauri::command]
fn elimina_catena(app: tauri::AppHandle, file: String) -> Result<(), String> {
    let root = workspace(&app)?;
    storage::elimina_catena(&root, &file).map_err(|e| e.to_string())
}

// ============================== Comandi Git ==============================

/// Elenco dei file con modifiche non committate.
#[tauri::command]
fn git_stato(app: tauri::AppHandle) -> Result<Vec<FileModificato>, String> {
    let root = workspace(&app)?;
    git::stato(&root).map_err(|e| e.to_string())
}

/// Diff (HEAD ↔ working dir) di un file, riga per riga.
#[tauri::command]
fn git_diff(app: tauri::AppHandle, file: String) -> Result<Vec<RigaDiff>, String> {
    let root = workspace(&app)?;
    git::diff_file(&root, &file).map_err(|e| e.to_string())
}

/// Commit dei file selezionati; restituisce lo SHA breve.
#[tauri::command]
fn git_commit(
    app: tauri::AppHandle,
    messaggio: String,
    files: Vec<String>,
) -> Result<String, String> {
    let root = workspace(&app)?;
    git::commit(&root, &messaggio, &files).map_err(|e| e.to_string())
}

/// Cronologia degli ultimi commit.
#[tauri::command]
fn git_log(app: tauri::AppHandle) -> Result<Vec<Commit>, String> {
    let root = workspace(&app)?;
    git::log(&root, 50).map_err(|e| e.to_string())
}

/// Stato del repo: branch, remote, ahead/behind.
#[tauri::command]
fn git_info(app: tauri::AppHandle) -> Result<StatoRepo, String> {
    let root = workspace(&app)?;
    git::info(&root).map_err(|e| e.to_string())
}

/// Imposta l'URL del remote "origin".
#[tauri::command]
fn git_imposta_remote(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let root = workspace(&app)?;
    git::imposta_remote(&root, &url).map_err(|e| e.to_string())
}

/// Esegue il pull da origin (fast-forward).
#[tauri::command]
fn git_pull(app: tauri::AppHandle) -> Result<String, String> {
    let root = workspace(&app)?;
    git::pull(&root).map_err(|e| e.to_string())
}

/// Esegue il push del branch corrente su origin.
#[tauri::command]
fn git_push(app: tauri::AppHandle) -> Result<String, String> {
    let root = workspace(&app)?;
    git::push(&root).map_err(|e| e.to_string())
}

/// Se il workspace è vuoto, crea una collezione di esempio così la UI non parte spoglia.
fn semina_esempio(app: &tauri::AppHandle) {
    let Ok(root) = workspace(app) else { return };
    let Ok(albero) = storage::carica_albero(&root) else { return };
    if !albero.is_empty() {
        return;
    }
    if let Ok(dir) = storage::crea_collezione(&root, "Esempi") {
        let _ = storage::crea_richiesta(&root, &dir, "Httpbin GET");
    }
}

/// Carica la config da disco e assicura che ci sia un workspace attivo (il default).
fn inizializza_config(app: &tauri::AppHandle) {
    let mut cfg = carica_config(app);
    if cfg.attivo.is_none() {
        if let Ok(def) = workspace_default(app) {
            let s = def.to_string_lossy().to_string();
            if !cfg.workspaces.contains(&s) {
                cfg.workspaces.insert(0, s.clone());
            }
            cfg.attivo = Some(s);
        }
    }
    *app.state::<StatoConfig>().0.lock().unwrap() = cfg.clone();
    let _ = salva_config(app, &cfg);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(StatoConfig(Mutex::new(Config::default())))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            inizializza_config(app.handle());
            semina_esempio(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            invia_richiesta,
            valuta_test,
            esegui_perf,
            lista_workspaces,
            aggiungi_workspace,
            imposta_workspace_attivo,
            rimuovi_workspace,
            percorso_workspace,
            carica_albero,
            salva_richiesta,
            crea_collezione,
            crea_cartella,
            crea_richiesta,
            elimina,
            rinomina_cartella,
            elimina_cartella,
            esporta_collezione,
            importa_collezione,
            carica_environments,
            salva_environment,
            elimina_environment,
            carica_catene,
            salva_catena,
            elimina_catena,
            git_stato,
            git_diff,
            git_commit,
            git_log,
            git_info,
            git_imposta_remote,
            git_pull,
            git_push,
        ])
        .run(tauri::generate_context!())
        .expect("errore durante l'avvio dell'applicazione Tauri");
}
