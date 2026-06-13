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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
