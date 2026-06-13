//! Server web di Rustman: espone gli stessi comandi dell'app desktop via REST
//! (sotto /api/<comando>) e serve il frontend compilato (cartella dist).
//!
//! Avvio:  RUSTMAN_WORKSPACE=./workspace RUSTMAN_DIST=./dist cargo run -p rustman-server
//! Poi apri http://localhost:1421

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use rustman_core::{
    curl, doc, git, http,
    model::{
        Asserzione, Auth, Catena, Environment, Richiesta, Risposta, VoceStoria,
    },
    oauth, perf, storage, test, textdiff, vars,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

/// Stato condiviso: la cartella del workspace (anche repo git).
#[derive(Clone)]
struct Stato {
    cfg: std::sync::Arc<std::sync::Mutex<WsConfig>>,
}

/// Config workspace lato server (in memoria; `attivo` è il root corrente).
#[derive(Clone, Default, serde::Serialize)]
struct WsConfig {
    workspaces: Vec<String>,
    attivo: String,
}

impl Stato {
    /// Cartella del workspace attivo (assicurando cartella + repo git).
    fn root(&self) -> PathBuf {
        let p = PathBuf::from(self.cfg.lock().unwrap().attivo.clone());
        let _ = std::fs::create_dir_all(&p);
        let _ = git::assicura_repo(&p);
        p
    }
}

/// Errore generico convertito in risposta HTTP 500 con il messaggio come testo.
struct Errore(String);
impl IntoResponse for Errore {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0).into_response()
    }
}
/// Helper per mappare qualsiasi errore "stampabile" in `Errore`.
fn err<E: std::fmt::Display>(e: E) -> Errore {
    Errore(e.to_string())
}

// ----------------------- Corpi delle richieste ------------------------------
// rename_all = camelCase così accettano lo stesso JSON che il frontend invia.

#[derive(Deserialize)]
struct InviaReq {
    richiesta: Richiesta,
    #[serde(default)]
    variabili: Option<HashMap<String, String>>,
}
#[derive(Deserialize)]
struct ValutaReq {
    asserzioni: Vec<Asserzione>,
    risposta: Risposta,
}
#[derive(Deserialize)]
struct PerfReq {
    richiesta: Richiesta,
    n: usize,
    concorrenza: usize,
    #[serde(default)]
    variabili: Option<HashMap<String, String>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SalvaRichiestaReq {
    dir: String,
    #[serde(default)]
    file_precedente: Option<String>,
    richiesta: Richiesta,
}
#[derive(Deserialize)]
struct NomeReq {
    nome: String,
}
#[derive(Deserialize)]
struct CreaRichiestaReq {
    dir: String,
    nome: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreaCartellaReq {
    dir_genitore: String,
    nome: String,
}
#[derive(Deserialize)]
struct FileReq {
    file: String,
}
#[derive(Deserialize)]
struct DirReq {
    dir: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RinominaReq {
    dir: String,
    nuovo_nome: String,
}
#[derive(Deserialize)]
struct UrlReq {
    url: String,
}
#[derive(Deserialize)]
struct ContenutoReq {
    contenuto: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SalvaEnvReq {
    #[serde(default)]
    file_precedente: Option<String>,
    environment: Environment,
}
#[derive(Deserialize)]
struct DiffReq {
    file: String,
}
#[derive(Deserialize)]
struct CommitReq {
    messaggio: String,
    files: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SalvaCatenaReq {
    #[serde(default)]
    file_precedente: Option<String>,
    catena: Catena,
}
#[derive(Deserialize)]
struct OauthReq {
    auth: Auth,
    #[serde(default)]
    variabili: Option<HashMap<String, String>>,
}
#[derive(Deserialize)]
struct CurlGenReq {
    richiesta: Richiesta,
}
#[derive(Deserialize)]
struct CurlImpReq {
    comando: String,
}
#[derive(Deserialize)]
struct StoriaReq {
    voce: VoceStoria,
}
#[derive(Deserialize)]
struct DiffTestiReq {
    vecchio: String,
    nuovo: String,
}
#[derive(Deserialize)]
struct AnteprimaReq {
    testo: String,
    #[serde(default)]
    variabili: Option<HashMap<String, String>>,
}

// ------------------------------ Handler -------------------------------------

async fn h_invia(State(s): State<Stato>, Json(r): Json<InviaReq>) -> Result<Json<Risposta>, Errore> {
    let req = match &r.variabili {
        Some(v) => vars::risolvi(&r.richiesta, v),
        None => r.richiesta,
    };
    let _ = &s; // la send non usa il workspace
    Ok(Json(http::invia(&req).await.map_err(err)?))
}

async fn h_valuta(Json(r): Json<ValutaReq>) -> Json<Vec<rustman_core::model::RisultatoTest>> {
    Json(test::valuta(&r.asserzioni, &r.risposta))
}

async fn h_perf(
    State(_s): State<Stato>,
    Json(r): Json<PerfReq>,
) -> Json<rustman_core::model::RisultatoPerf> {
    let req = match &r.variabili {
        Some(v) => vars::risolvi(&r.richiesta, v),
        None => r.richiesta,
    };
    Json(perf::esegui(&req, r.n, r.concorrenza).await)
}

async fn h_oauth_token(Json(r): Json<OauthReq>) -> Result<Json<String>, Errore> {
    let mut cfg = r.auth.oauth2;
    if let Some(v) = &r.variabili {
        let s = |t: &str| vars::sostituisci(t, v);
        cfg.token_url = s(&cfg.token_url);
        cfg.client_id = s(&cfg.client_id);
        cfg.client_secret = s(&cfg.client_secret);
        cfg.username = s(&cfg.username);
        cfg.password = s(&cfg.password);
        cfg.scope = s(&cfg.scope);
    }
    Ok(Json(oauth::ottieni_token(&cfg).await.map_err(err)?))
}

async fn h_genera_curl(Json(r): Json<CurlGenReq>) -> Json<String> {
    Json(curl::genera(&r.richiesta))
}

async fn h_importa_curl(Json(r): Json<CurlImpReq>) -> Result<Json<Richiesta>, Errore> {
    curl::analizza(&r.comando)
        .map(Json)
        .ok_or_else(|| Errore("comando cURL non riconosciuto".into()))
}

async fn h_carica_storia(State(s): State<Stato>) -> Json<Vec<VoceStoria>> {
    Json(storage::carica_storia(&s.root()))
}

async fn h_aggiungi_storia(
    State(s): State<Stato>,
    Json(r): Json<StoriaReq>,
) -> Result<Json<()>, Errore> {
    storage::aggiungi_storia(&s.root(), r.voce).map_err(err)?;
    Ok(Json(()))
}

async fn h_pulisci_storia(State(s): State<Stato>) -> Result<Json<()>, Errore> {
    storage::pulisci_storia(&s.root()).map_err(err)?;
    Ok(Json(()))
}

async fn h_diff_testi(
    Json(r): Json<DiffTestiReq>,
) -> Json<Vec<rustman_core::model::RigaDiff>> {
    Json(textdiff::diff_linee(&r.vecchio, &r.nuovo))
}

async fn h_genera_doc(State(s): State<Stato>) -> Result<Json<String>, Errore> {
    let albero = storage::carica_albero(&s.root()).map_err(err)?;
    Ok(Json(doc::genera(&albero)))
}

async fn h_anteprima(Json(r): Json<AnteprimaReq>) -> Json<String> {
    Json(vars::sostituisci(&r.testo, &r.variabili.unwrap_or_default()))
}

async fn h_percorso(State(s): State<Stato>) -> Json<String> {
    Json(s.root().to_string_lossy().to_string())
}

async fn h_carica_albero(
    State(s): State<Stato>,
) -> Result<Json<rustman_core::model::Albero>, Errore> {
    Ok(Json(storage::carica_albero(&s.root()).map_err(err)?))
}

async fn h_salva_richiesta(
    State(s): State<Stato>,
    Json(r): Json<SalvaRichiestaReq>,
) -> Result<Json<String>, Errore> {
    Ok(Json(
        storage::salva_richiesta(&s.root(), &r.dir, r.file_precedente.as_deref(), &r.richiesta)
            .map_err(err)?,
    ))
}

async fn h_crea_collezione(
    State(s): State<Stato>,
    Json(r): Json<NomeReq>,
) -> Result<Json<String>, Errore> {
    Ok(Json(storage::crea_collezione(&s.root(), &r.nome).map_err(err)?))
}

async fn h_crea_richiesta(
    State(s): State<Stato>,
    Json(r): Json<CreaRichiestaReq>,
) -> Result<Json<String>, Errore> {
    Ok(Json(
        storage::crea_richiesta(&s.root(), &r.dir, &r.nome).map_err(err)?,
    ))
}

async fn h_crea_cartella(
    State(s): State<Stato>,
    Json(r): Json<CreaCartellaReq>,
) -> Result<Json<String>, Errore> {
    Ok(Json(
        storage::crea_cartella(&s.root(), &r.dir_genitore, &r.nome).map_err(err)?,
    ))
}

async fn h_elimina(State(s): State<Stato>, Json(r): Json<FileReq>) -> Result<Json<()>, Errore> {
    storage::elimina(&s.root(), &r.file).map_err(err)?;
    Ok(Json(()))
}

async fn h_rinomina_cartella(
    State(s): State<Stato>,
    Json(r): Json<RinominaReq>,
) -> Result<Json<String>, Errore> {
    Ok(Json(
        storage::rinomina_cartella(&s.root(), &r.dir, &r.nuovo_nome).map_err(err)?,
    ))
}

async fn h_elimina_cartella(
    State(s): State<Stato>,
    Json(r): Json<DirReq>,
) -> Result<Json<()>, Errore> {
    storage::elimina_cartella(&s.root(), &r.dir).map_err(err)?;
    Ok(Json(()))
}

async fn h_esporta(State(s): State<Stato>, Json(r): Json<DirReq>) -> Result<Json<String>, Errore> {
    Ok(Json(storage::esporta_collezione(&s.root(), &r.dir).map_err(err)?))
}

async fn h_importa(
    State(s): State<Stato>,
    Json(r): Json<ContenutoReq>,
) -> Result<Json<String>, Errore> {
    Ok(Json(
        storage::importa_collezione(&s.root(), &r.contenuto).map_err(err)?,
    ))
}

async fn h_importa_smart(
    State(s): State<Stato>,
    Json(r): Json<ContenutoReq>,
) -> Result<Json<rustman_core::model::RisultatoImport>, Errore> {
    Ok(Json(storage::importa(&s.root(), &r.contenuto).map_err(err)?))
}

async fn h_carica_env(
    State(s): State<Stato>,
) -> Result<Json<Vec<rustman_core::model::EnvironmentSuDisco>>, Errore> {
    Ok(Json(storage::carica_environments(&s.root()).map_err(err)?))
}

async fn h_salva_env(
    State(s): State<Stato>,
    Json(r): Json<SalvaEnvReq>,
) -> Result<Json<String>, Errore> {
    Ok(Json(
        storage::salva_environment(&s.root(), r.file_precedente.as_deref(), &r.environment)
            .map_err(err)?,
    ))
}

async fn h_elimina_env(State(s): State<Stato>, Json(r): Json<FileReq>) -> Result<Json<()>, Errore> {
    storage::elimina_environment(&s.root(), &r.file).map_err(err)?;
    Ok(Json(()))
}

async fn h_carica_catene(
    State(s): State<Stato>,
) -> Result<Json<Vec<rustman_core::model::CatenaSuDisco>>, Errore> {
    Ok(Json(storage::carica_catene(&s.root()).map_err(err)?))
}

async fn h_salva_catena(
    State(s): State<Stato>,
    Json(r): Json<SalvaCatenaReq>,
) -> Result<Json<String>, Errore> {
    Ok(Json(
        storage::salva_catena(&s.root(), r.file_precedente.as_deref(), &r.catena).map_err(err)?,
    ))
}

async fn h_elimina_catena(State(s): State<Stato>, Json(r): Json<FileReq>) -> Result<Json<()>, Errore> {
    storage::elimina_catena(&s.root(), &r.file).map_err(err)?;
    Ok(Json(()))
}

async fn h_git_stato(
    State(s): State<Stato>,
) -> Result<Json<Vec<rustman_core::model::FileModificato>>, Errore> {
    Ok(Json(git::stato(&s.root()).map_err(err)?))
}

async fn h_git_diff(
    State(s): State<Stato>,
    Json(r): Json<DiffReq>,
) -> Result<Json<Vec<rustman_core::model::RigaDiff>>, Errore> {
    Ok(Json(git::diff_file(&s.root(), &r.file).map_err(err)?))
}

async fn h_git_commit(
    State(s): State<Stato>,
    Json(r): Json<CommitReq>,
) -> Result<Json<String>, Errore> {
    Ok(Json(git::commit(&s.root(), &r.messaggio, &r.files).map_err(err)?))
}

async fn h_git_log(
    State(s): State<Stato>,
) -> Result<Json<Vec<rustman_core::model::Commit>>, Errore> {
    Ok(Json(git::log(&s.root(), 50).map_err(err)?))
}

async fn h_git_info(
    State(s): State<Stato>,
) -> Result<Json<rustman_core::model::StatoRepo>, Errore> {
    Ok(Json(git::info(&s.root()).map_err(err)?))
}

async fn h_git_remote(State(s): State<Stato>, Json(r): Json<UrlReq>) -> Result<Json<()>, Errore> {
    git::imposta_remote(&s.root(), &r.url).map_err(err)?;
    Ok(Json(()))
}

async fn h_git_pull(State(s): State<Stato>) -> Result<Json<String>, Errore> {
    Ok(Json(git::pull(&s.root()).map_err(err)?))
}

async fn h_git_push(State(s): State<Stato>) -> Result<Json<String>, Errore> {
    Ok(Json(git::push(&s.root()).map_err(err)?))
}

// ----------------------------- Workspaces -----------------------------------

#[derive(Deserialize)]
struct PercorsoReq {
    percorso: String,
}

async fn h_lista_workspaces(State(s): State<Stato>) -> Json<WsConfig> {
    Json(s.cfg.lock().unwrap().clone())
}

async fn h_aggiungi_workspace(State(s): State<Stato>, Json(r): Json<PercorsoReq>) -> Json<()> {
    let mut c = s.cfg.lock().unwrap();
    if !c.workspaces.contains(&r.percorso) {
        c.workspaces.push(r.percorso);
    }
    Json(())
}

async fn h_imposta_workspace(State(s): State<Stato>, Json(r): Json<PercorsoReq>) -> Json<()> {
    {
        let mut c = s.cfg.lock().unwrap();
        if !c.workspaces.contains(&r.percorso) {
            c.workspaces.push(r.percorso.clone());
        }
        c.attivo = r.percorso;
    }
    s.root(); // assicura cartella + repo della nuova attiva
    Json(())
}

async fn h_rimuovi_workspace(State(s): State<Stato>, Json(r): Json<PercorsoReq>) -> Json<()> {
    let mut c = s.cfg.lock().unwrap();
    c.workspaces.retain(|p| p != &r.percorso);
    if c.attivo == r.percorso {
        c.attivo = c.workspaces.first().cloned().unwrap_or_default();
    }
    Json(())
}

#[tokio::main]
async fn main() {
    // Cartella del workspace (default ./workspace) e della dist (default ./dist).
    let root = PathBuf::from(std::env::var("RUSTMAN_WORKSPACE").unwrap_or_else(|_| "workspace".into()));
    let dist = std::env::var("RUSTMAN_DIST").unwrap_or_else(|_| "dist".into());

    std::fs::create_dir_all(&root).expect("impossibile creare il workspace");
    git::assicura_repo(&root).expect("impossibile inizializzare il repo git");

    let attivo = root.to_string_lossy().to_string();
    let stato = Stato {
        cfg: std::sync::Arc::new(std::sync::Mutex::new(WsConfig {
            workspaces: vec![attivo.clone()],
            attivo,
        })),
    };

    // Serve la dist; per la SPA, le rotte sconosciute ricadono su index.html.
    let statici =
        ServeDir::new(&dist).not_found_service(ServeFile::new(format!("{}/index.html", dist)));

    let app = Router::new()
        .route("/api/invia_richiesta", post(h_invia))
        .route("/api/oauth2_token", post(h_oauth_token))
        .route("/api/genera_curl", post(h_genera_curl))
        .route("/api/importa_curl", post(h_importa_curl))
        .route("/api/valuta_test", post(h_valuta))
        .route("/api/esegui_perf", post(h_perf))
        .route("/api/lista_workspaces", post(h_lista_workspaces))
        .route("/api/aggiungi_workspace", post(h_aggiungi_workspace))
        .route("/api/imposta_workspace_attivo", post(h_imposta_workspace))
        .route("/api/rimuovi_workspace", post(h_rimuovi_workspace))
        .route("/api/percorso_workspace", post(h_percorso))
        .route("/api/carica_albero", post(h_carica_albero))
        .route("/api/salva_richiesta", post(h_salva_richiesta))
        .route("/api/crea_collezione", post(h_crea_collezione))
        .route("/api/crea_cartella", post(h_crea_cartella))
        .route("/api/crea_richiesta", post(h_crea_richiesta))
        .route("/api/elimina", post(h_elimina))
        .route("/api/rinomina_cartella", post(h_rinomina_cartella))
        .route("/api/elimina_cartella", post(h_elimina_cartella))
        .route("/api/esporta_collezione", post(h_esporta))
        .route("/api/importa_collezione", post(h_importa))
        .route("/api/importa", post(h_importa_smart))
        .route("/api/carica_environments", post(h_carica_env))
        .route("/api/salva_environment", post(h_salva_env))
        .route("/api/elimina_environment", post(h_elimina_env))
        .route("/api/carica_catene", post(h_carica_catene))
        .route("/api/salva_catena", post(h_salva_catena))
        .route("/api/elimina_catena", post(h_elimina_catena))
        .route("/api/carica_storia", post(h_carica_storia))
        .route("/api/aggiungi_storia", post(h_aggiungi_storia))
        .route("/api/pulisci_storia", post(h_pulisci_storia))
        .route("/api/diff_testi", post(h_diff_testi))
        .route("/api/genera_doc", post(h_genera_doc))
        .route("/api/anteprima", post(h_anteprima))
        .route("/api/git_stato", post(h_git_stato))
        .route("/api/git_diff", post(h_git_diff))
        .route("/api/git_commit", post(h_git_commit))
        .route("/api/git_log", post(h_git_log))
        .route("/api/git_info", post(h_git_info))
        .route("/api/git_imposta_remote", post(h_git_remote))
        .route("/api/git_pull", post(h_git_pull))
        .route("/api/git_push", post(h_git_push))
        .fallback_service(statici)
        .layer(CorsLayer::permissive())
        .with_state(stato);

    let addr = "0.0.0.0:1421";
    println!("Rustman server su http://localhost:1421 (workspace + dist serviti)");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind fallito");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("server terminato con errore");
}
