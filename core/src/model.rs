//! Modelli dati condivisi tra backend e frontend.
//! I nomi dei campi sono in italiano e vengono usati così anche dal frontend.

use serde::{Deserialize, Serialize};

/// Una singola intestazione HTTP (chiave/valore) con flag di abilitazione.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub chiave: String,
    pub valore: String,
    /// Se false, l'intestazione viene ignorata all'invio.
    #[serde(default = "vero")]
    pub attivo: bool,
}

/// Valore di default per il campo `attivo` quando manca nel JSON.
fn vero() -> bool {
    true
}

/// Modalità del corpo di default (compatibilità con i file salvati prima).
fn body_raw() -> String {
    "raw".to_string()
}

fn campo_text() -> String {
    "text".to_string()
}

/// Un campo di un form (`form-data` o `x-www-form-urlencoded`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampoForm {
    pub chiave: String,
    /// Valore testuale (per i campi di tipo "text").
    #[serde(default)]
    pub valore: String,
    /// Tipo del campo: "text" oppure "file".
    #[serde(default = "campo_text")]
    pub tipo: String,
    /// Percorso del file da inviare (solo per `tipo == "file"`, solo desktop).
    #[serde(default)]
    pub file: String,
    #[serde(default = "vero")]
    pub attivo: bool,
}

/// La richiesta HTTP che l'utente vuole inviare (e che salviamo su file).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Richiesta {
    /// Nome leggibile della richiesta (es. "Login"). Usato anche per il nome file.
    #[serde(default)]
    pub nome: String,
    /// Metodo HTTP: GET, POST, PUT, DELETE, ...
    pub metodo: String,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<Header>,
    /// Parametri della query string (?chiave=valore). Stesso formato degli header.
    #[serde(default)]
    pub params: Vec<Header>,
    /// Autenticazione (none/bearer/basic).
    #[serde(default)]
    pub auth: Auth,
    /// Corpo grezzo della richiesta (es. testo JSON). Vuoto = nessun corpo.
    /// Usato quando `body_mode` è "raw".
    #[serde(default)]
    pub body: String,
    /// Modalità del corpo: "raw" | "form-data" | "x-www-form-urlencoded".
    #[serde(default = "body_raw")]
    pub body_mode: String,
    /// Campi del form (per "form-data" e "x-www-form-urlencoded").
    #[serde(default)]
    pub form: Vec<CampoForm>,
    /// Asserzioni da verificare sulla risposta (Fase 4).
    #[serde(default)]
    pub tests: Vec<Asserzione>,
    /// Script JavaScript eseguito PRIMA dell'invio (può modificare la richiesta/variabili).
    #[serde(default)]
    pub pre_script: String,
    /// Script JavaScript eseguito DOPO la risposta (test/variabili).
    #[serde(default)]
    pub post_script: String,
    /// Impostazioni di rete della richiesta (timeout, redirect, TLS, retry 429).
    #[serde(default)]
    pub impostazioni: Impostazioni,
    /// Etichette per organizzare/filtrare e per le suite (`--tag` nella CLI).
    #[serde(default)]
    pub tags: Vec<String>,
    /// Descrizione in Markdown (mostrata nella doc generata).
    #[serde(default)]
    pub descrizione: String,
}

/// Impostazioni di rete per-richiesta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Impostazioni {
    /// Timeout in millisecondi (0 = nessun timeout esplicito).
    #[serde(default)]
    pub timeout_ms: u64,
    /// Se seguire i redirect HTTP (default true).
    #[serde(default = "vero")]
    pub segui_redirect: bool,
    /// Se verificare il certificato TLS (default true).
    #[serde(default = "vero")]
    pub verifica_tls: bool,
    /// Numero di ritentativi automatici su risposta 429 (Too Many Requests),
    /// rispettando l'header `Retry-After`. 0 = disattivato.
    #[serde(default)]
    pub retry_429: u32,
}

impl Default for Impostazioni {
    fn default() -> Self {
        Impostazioni {
            timeout_ms: 0,
            segui_redirect: true,
            verifica_tls: true,
            retry_429: 0,
        }
    }
}

/// Autenticazione della richiesta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Auth {
    /// "none" | "bearer" | "basic" | "oauth2".
    #[serde(default = "auth_none")]
    pub tipo: String,
    /// Token per il tipo "bearer".
    #[serde(default)]
    pub token: String,
    /// Utente e password per il tipo "basic".
    #[serde(default)]
    pub utente: String,
    #[serde(default)]
    pub password: String,
    /// Configurazione OAuth2 (usata quando `tipo == "oauth2"`).
    #[serde(default)]
    pub oauth2: Oauth2,
}

/// Parametri OAuth2. L'`access_token` viene ottenuto da `token_url` (grant
/// client_credentials/password) oppure incollato a mano, e poi inviato come Bearer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Oauth2 {
    /// "client_credentials" | "password" | "authorization_code".
    #[serde(default)]
    pub grant_type: String,
    #[serde(default)]
    pub token_url: String,
    /// URL di autorizzazione (per il grant authorization_code, gestito dalla UI).
    #[serde(default)]
    pub auth_url: String,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub scope: String,
    /// Token corrente da inviare come `Authorization: Bearer ...`.
    #[serde(default)]
    pub access_token: String,
}

fn auth_none() -> String {
    "none".to_string()
}

impl Default for Auth {
    fn default() -> Self {
        Auth {
            tipo: auth_none(),
            token: String::new(),
            utente: String::new(),
            password: String::new(),
            oauth2: Oauth2::default(),
        }
    }
}

/// La risposta ricevuta dal server, con le metriche utili alla UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Risposta {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<Header>,
    pub body: String,
    /// Durata totale della richiesta in millisecondi.
    pub tempo_ms: u128,
    /// Dimensione del corpo della risposta in byte.
    pub dimensione: usize,
}

// ======================== Collection su file (Fase 2) ========================

/// Nodo dell'albero di una collezione: una sottocartella o una richiesta.
/// Serializzato con un campo discriminante `tipo` ("cartella" | "richiesta").
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tipo", rename_all = "lowercase")]
pub enum Nodo {
    Cartella {
        nome: String,
        /// Percorso della cartella relativo alla root (es. "test/auth").
        dir: String,
        figli: Vec<Nodo>,
    },
    Richiesta {
        nome: String,
        /// Percorso del file relativo alla root (es. "test/auth/login.json").
        file: String,
        richiesta: Richiesta,
    },
}

/// Configurazione ereditabile di una cartella/collezione: header e auth applicati
/// alle richieste figlie (salvata in `<cartella>/_rustman.json`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfigCartella {
    #[serde(default)]
    pub headers: Vec<Header>,
    #[serde(default)]
    pub auth: Auth,
    /// Variabili di collezione/cartella (priorità inferiore all'ambiente attivo).
    #[serde(default)]
    pub variabili: Vec<Variabile>,
}

/// Una collezione = una cartella di primo livello, con i suoi figli (cartelle/richieste).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collezione {
    pub nome: String,
    /// Percorso della cartella relativo alla root (es. "User APIs").
    pub dir: String,
    pub figli: Vec<Nodo>,
}

/// L'intero albero del workspace: l'elenco delle collezioni.
pub type Albero = Vec<Collezione>;

// ===================== Run / catene di chiamate ==============================

/// Un passo di una catena: riferimento a una richiesta (per percorso file).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Passo {
    pub file: String,
}

/// Una catena di chiamate da eseguire in sequenza (integration test).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catena {
    pub nome: String,
    #[serde(default)]
    pub passi: Vec<Passo>,
}

/// Una catena con il percorso del file da cui è stata caricata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatenaSuDisco {
    pub file: String,
    pub catena: Catena,
}

/// Stato del repository git del workspace (per la vista Git).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatoRepo {
    pub branch: Option<String>,
    pub remote: Option<String>,
    /// Commit locali non ancora inviati (ahead) e remoti non ancora presi (behind).
    pub ahead: usize,
    pub behind: usize,
}

// ===================== Environments / variabili ==============================

/// Una variabile d'ambiente (es. base_url = https://api.example.com).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variabile {
    pub chiave: String,
    pub valore: String,
    /// Se true, il valore è un segreto: non viene scritto nel file committato
    /// in git, ma in un archivio separato (`.rustman-secrets.json`, gitignorato).
    #[serde(default)]
    pub segreto: bool,
}

/// Un ambiente: un insieme di variabili usate per sostituire i {{segnaposto}}.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub nome: String,
    #[serde(default)]
    pub variabili: Vec<Variabile>,
}

/// Un ambiente con il percorso del file da cui è stato caricato.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSuDisco {
    pub file: String,
    pub environment: Environment,
}

// ===================== Import / Export ======================================

/// Nodo dell'albero usato nel formato di esportazione (senza percorsi su disco).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tipo", rename_all = "lowercase")]
pub enum NodoExport {
    Cartella { nome: String, figli: Vec<NodoExport> },
    Richiesta { richiesta: Richiesta },
}

/// Formato portabile per esportare/importare una collezione (con le sottocartelle).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EsportaCollezione {
    /// Versione del formato (per compatibilità futura).
    #[serde(default = "versione_uno")]
    pub rustman: u32,
    pub nome: String,
    #[serde(default)]
    pub figli: Vec<NodoExport>,
}

fn versione_uno() -> u32 {
    1
}

/// Riassunto di un'esecuzione di test, per il trend storico del pass-rate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    /// Data/ora ISO dell'esecuzione.
    pub quando: String,
    pub totali: usize,
    pub ok: usize,
    pub ko: usize,
    /// Etichetta (es. nome della richiesta o della catena).
    #[serde(default)]
    pub etichetta: String,
}

/// Report di copertura: quali operazioni dello spec OpenAPI hanno (o no)
/// una richiesta con asserzioni.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoverageReport {
    pub totali: usize,
    pub coperti: usize,
    pub scoperti: Vec<String>,
    pub percentuale: f64,
}

/// Esito dell'esecuzione di una richiesta in un run (per il report HTML).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RisultatoRun {
    pub nome: String,
    pub metodo: String,
    pub url: String,
    pub status: u16,
    pub status_text: String,
    pub tempo_ms: u128,
    /// Messaggio d'errore se l'invio è fallito (vuoto altrimenti).
    #[serde(default)]
    pub errore: String,
    #[serde(default)]
    pub tests: Vec<RisultatoTest>,
}

/// Un cookie visto in una risposta (per il cookie inspector).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieInfo {
    pub dominio: String,
    pub nome: String,
    pub valore: String,
    pub attributi: String,
}

/// Un avviso del security scan sulla risposta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAvviso {
    /// "alto" | "medio" | "info".
    pub livello: String,
    pub titolo: String,
    pub dettaglio: String,
}

/// Una rotta del mock server: metodo + path (templato) e la risposta canned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockRoute {
    pub metodo: String,
    /// Path OpenAPI, es. "/pets/{petId}".
    pub path: String,
    pub status: u16,
    pub body: String,
    pub content_type: String,
}

/// Report del confronto fra due spec OpenAPI (drift detection).
/// Ogni voce è una stringa tipo "GET /pets".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DriftReport {
    /// Operazioni presenti solo nel nuovo spec.
    pub aggiunti: Vec<String>,
    /// Operazioni presenti solo nel vecchio spec.
    pub rimossi: Vec<String>,
    /// Operazioni presenti in entrambi ma con parametri/corpo cambiati.
    pub modificati: Vec<String>,
}

/// Esito di un import: dice al frontend cosa è stato creato, così può
/// ricaricare l'albero delle collezioni o la lista degli ambienti.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tipo", rename_all = "lowercase")]
pub enum RisultatoImport {
    /// Importata una collezione: percorso della cartella creata. Se la collezione
    /// Postman aveva delle variabili, `environment` riporta il file dell'ambiente
    /// creato a partire da quelle.
    Collezione {
        dir: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        environment: Option<String>,
    },
    /// Importato un ambiente: percorso del file creato.
    Environment { file: String },
}

// ============================ Git (Fase 3) ===================================

/// Un file con modifiche non ancora committate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileModificato {
    /// Percorso relativo alla root del workspace.
    pub file: String,
    /// Stato: "M" modificato, "A" aggiunto/nuovo, "D" eliminato.
    pub stato: String,
}

/// Una singola riga di un diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RigaDiff {
    /// Tipo riga: "ctx" (contesto), "add" (aggiunta), "rem" (rimozione).
    pub tipo: String,
    pub testo: String,
    /// Numero di riga nella versione vecchia (HEAD), se presente.
    pub vecchia: Option<u32>,
    /// Numero di riga nella versione nuova (working dir), se presente.
    pub nuova: Option<u32>,
}

/// Un commit nella cronologia.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub sha: String,
    pub messaggio: String,
    pub autore: String,
    /// Data/ora in formato leggibile.
    pub quando: String,
}

// ============================ Test (Fase 4) ==================================

/// Una singola asserzione da verificare sulla risposta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asserzione {
    /// Cosa controllare: "status" | "tempo" | "header" | "body" | "json".
    pub tipo: String,
    /// Operatore di confronto: "==" | "!=" | "<" | ">" | "contiene".
    pub operatore: String,
    /// Per "header" è il nome dell'header; per "json" è il path (es. "data.id");
    /// per gli altri tipi è vuoto.
    #[serde(default)]
    pub campo: String,
    /// Valore atteso (sempre come stringa).
    pub atteso: String,
    #[serde(default = "vero")]
    pub attivo: bool,
}

/// Esito della verifica di una singola asserzione.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RisultatoTest {
    /// Descrizione leggibile dell'asserzione (es. "status == 200").
    pub descrizione: String,
    pub passato: bool,
    /// Dettaglio dell'esito (es. "ottenuto 404").
    pub dettaglio: String,
}

// ========================= Performance (Fase 5) ==============================

/// Risultato aggregato di un test di carico.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RisultatoPerf {
    pub totali: usize,
    pub ok: usize,
    pub errori: usize,
    /// Durata complessiva del test in millisecondi.
    pub durata_totale_ms: u128,
    pub req_al_secondo: f64,
    pub latenza_min: u128,
    pub latenza_max: u128,
    pub latenza_media: f64,
    pub p50: u128,
    pub p90: u128,
    pub p95: u128,
    pub p99: u128,
    /// Tutte le latenze (ms) in ordine di completamento, per i grafici.
    pub latenze: Vec<u128>,
}

/// Opzioni di un test di carico. Modo "count" (n richieste) o "durata" (per
/// `durata_s` secondi), con eventuale RPS target e warmup scartato.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpzioniPerf {
    #[serde(default = "uno")]
    pub concorrenza: usize,
    /// Numero di richieste (modo count). Ignorato se `durata_s > 0`.
    #[serde(default)]
    pub n: usize,
    /// Durata del test in secondi (modo durata). 0 = usa `n`.
    #[serde(default)]
    pub durata_s: u64,
    /// Richieste al secondo target (0 = massimo possibile).
    #[serde(default)]
    pub rps: u64,
    /// Secondi di warmup iniziale, esclusi dalle statistiche.
    #[serde(default)]
    pub warmup_s: u64,
    /// Profilo di carico: "costante" | "spike" | "soak".
    #[serde(default = "profilo_costante")]
    pub profilo: String,
    /// RPS durante la fase di picco (solo profilo "spike").
    #[serde(default)]
    pub spike_rps: u64,
}

fn uno() -> usize {
    1
}

fn profilo_costante() -> String {
    "costante".to_string()
}

impl Default for OpzioniPerf {
    fn default() -> Self {
        OpzioniPerf {
            concorrenza: 1,
            n: 0,
            durata_s: 0,
            rps: 0,
            warmup_s: 0,
            profilo: profilo_costante(),
            spike_rps: 0,
        }
    }
}

// ========================= History / replay =================================

/// Una voce della cronologia delle richieste inviate (per la vista History).
/// Contiene la richiesta completa così com'è stata inviata, per poterla
/// rieseguire ("replay"), più un riassunto della risposta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoceStoria {
    /// Data/ora ISO dell'invio (es. "2024-01-01T12:00:00Z").
    pub quando: String,
    /// La richiesta inviata (con le variabili già risolte).
    pub richiesta: Richiesta,
    pub status: u16,
    pub status_text: String,
    pub tempo_ms: u128,
    pub dimensione: usize,
    /// Corpo della risposta (eventualmente troncato), per il diff fra due voci.
    #[serde(default)]
    pub body: String,
    /// Nome dell'ambiente attivo al momento dell'invio (se presente).
    #[serde(default)]
    pub ambiente: String,
}
