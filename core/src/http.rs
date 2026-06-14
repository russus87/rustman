//! Client HTTP: invia una `Richiesta` e misura il tempo di risposta.

use crate::model::{Header, Richiesta, Risposta};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

/// Cookie jar condiviso per la durata del processo: i `Set-Cookie` ricevuti
/// vengono rimandati automaticamente alle richieste successive (sessioni).
fn cookie_jar() -> Arc<reqwest::cookie::Jar> {
    static JAR: OnceLock<Arc<reqwest::cookie::Jar>> = OnceLock::new();
    JAR.get_or_init(|| Arc::new(reqwest::cookie::Jar::default()))
        .clone()
}

/// Costruisce il client applicando le impostazioni della richiesta
/// (timeout, redirect, verifica TLS) e il cookie jar condiviso.
fn costruisci_client(richiesta: &Richiesta) -> Result<reqwest::Client, ErroreHttp> {
    let imp = &richiesta.impostazioni;
    let mut b = reqwest::Client::builder().cookie_provider(cookie_jar());
    if imp.timeout_ms > 0 {
        b = b.timeout(Duration::from_millis(imp.timeout_ms));
    }
    if !imp.segui_redirect {
        b = b.redirect(reqwest::redirect::Policy::none());
    }
    if !imp.verifica_tls {
        b = b.danger_accept_invalid_certs(true);
    }
    b.build().map_err(ErroreHttp::Rete)
}

/// Errori possibili durante l'invio di una richiesta.
#[derive(Debug, thiserror::Error)]
pub enum ErroreHttp {
    #[error("metodo HTTP non valido: {0}")]
    MetodoNonValido(String),
    #[error("errore di rete: {0}")]
    Rete(#[from] reqwest::Error),
    #[error("impossibile leggere il file da allegare: {0}")]
    File(#[from] std::io::Error),
    #[error("OAuth2: {0}")]
    Oauth(String),
}

/// Invia la richiesta e restituisce la risposta con le metriche raccolte.
pub async fn invia(richiesta: &Richiesta) -> Result<Risposta, ErroreHttp> {
    // Converte la stringa del metodo (es. "POST") nel tipo di reqwest.
    let metodo = reqwest::Method::from_bytes(richiesta.metodo.as_bytes())
        .map_err(|_| ErroreHttp::MetodoNonValido(richiesta.metodo.clone()))?;

    let client = costruisci_client(richiesta)?;

    // Misura il tempo totale: dall'invio alla ricezione del corpo.
    let inizio = Instant::now();

    // Loop di invio: in caso di 429 (Too Many Requests) rispetta `Retry-After`
    // e riprova fino a `retry_429` volte.
    let mut tentativo = 0u32;
    let resp = loop {
        let req = costruisci_req(&client, metodo.clone(), richiesta)?;
        let resp = req.send().await?;
        if resp.status().as_u16() == 429 && tentativo < richiesta.impostazioni.retry_429 {
            tentativo += 1;
            let attesa = retry_after(&resp).unwrap_or(1);
            tokio::time::sleep(Duration::from_secs(attesa)).await;
            continue;
        }
        break resp;
    };
    let status = resp.status();

    // Copia le intestazioni della risposta nel nostro modello.
    let headers = resp
        .headers()
        .iter()
        .map(|(k, v)| Header {
            chiave: k.to_string(),
            valore: v.to_str().unwrap_or("").to_string(),
            attivo: true,
        })
        .collect();

    let body = resp.text().await?;
    let tempo_ms = inizio.elapsed().as_millis();

    Ok(Risposta {
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("").to_string(),
        dimensione: body.len(),
        headers,
        body,
        tempo_ms,
    })
}

/// Costruisce la `RequestBuilder` con query, header, auth e corpo. Estratta per
/// poter ricostruire la richiesta a ogni tentativo (retry 429).
fn costruisci_req(
    client: &reqwest::Client,
    metodo: reqwest::Method,
    richiesta: &Richiesta,
) -> Result<reqwest::RequestBuilder, ErroreHttp> {
    let mut req = client.request(metodo, &richiesta.url);

    let query: Vec<(&str, &str)> = richiesta
        .params
        .iter()
        .filter(|p| p.attivo && !p.chiave.is_empty())
        .map(|p| (p.chiave.as_str(), p.valore.as_str()))
        .collect();
    if !query.is_empty() {
        req = req.query(&query);
    }

    for h in &richiesta.headers {
        if h.attivo && !h.chiave.is_empty() {
            req = req.header(&h.chiave, &h.valore);
        }
    }

    match richiesta.auth.tipo.as_str() {
        "bearer" if !richiesta.auth.token.is_empty() => {
            req = req.bearer_auth(&richiesta.auth.token);
        }
        "basic" => {
            let pwd = if richiesta.auth.password.is_empty() {
                None
            } else {
                Some(&richiesta.auth.password)
            };
            req = req.basic_auth(&richiesta.auth.utente, pwd);
        }
        "oauth2" if !richiesta.auth.oauth2.access_token.is_empty() => {
            req = req.bearer_auth(&richiesta.auth.oauth2.access_token);
        }
        _ => {}
    }

    applica_corpo(req, richiesta)
}

/// Estrae i secondi di attesa dall'header `Retry-After` (formato in secondi).
fn retry_after(resp: &reqwest::Response) -> Option<u64> {
    resp.headers()
        .get("retry-after")?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
}

/// Applica il corpo alla richiesta secondo `body_mode`:
/// - "x-www-form-urlencoded": invia i campi testo come form urlencoded;
/// - "form-data": costruisce un multipart con campi testo e file (letti da disco);
/// - "raw" (o altro): invia il corpo grezzo così com'è.
fn applica_corpo(
    req: reqwest::RequestBuilder,
    richiesta: &Richiesta,
) -> Result<reqwest::RequestBuilder, ErroreHttp> {
    match richiesta.body_mode.as_str() {
        "x-www-form-urlencoded" => {
            let coppie: Vec<(&str, &str)> = richiesta
                .form
                .iter()
                .filter(|c| c.attivo && !c.chiave.is_empty() && c.tipo != "file")
                .map(|c| (c.chiave.as_str(), c.valore.as_str()))
                .collect();
            Ok(if coppie.is_empty() {
                req
            } else {
                req.form(&coppie)
            })
        }
        "form-data" => {
            let mut form = reqwest::multipart::Form::new();
            let mut vuoto = true;
            for c in &richiesta.form {
                if !c.attivo || c.chiave.is_empty() {
                    continue;
                }
                if c.tipo == "file" {
                    if c.file.is_empty() {
                        continue;
                    }
                    // I file vengono letti dal filesystem locale (solo desktop).
                    let bytes = std::fs::read(&c.file)?;
                    let nome_file = std::path::Path::new(&c.file)
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "file".to_string());
                    let part = reqwest::multipart::Part::bytes(bytes).file_name(nome_file);
                    form = form.part(c.chiave.clone(), part);
                } else {
                    form = form.text(c.chiave.clone(), c.valore.clone());
                }
                vuoto = false;
            }
            Ok(if vuoto { req } else { req.multipart(form) })
        }
        _ => Ok(if richiesta.body.is_empty() {
            req
        } else {
            req.body(richiesta.body.clone())
        }),
    }
}
