//! Client HTTP: invia una `Richiesta` e misura il tempo di risposta.

use crate::model::{Header, Richiesta, Risposta};
use std::time::Instant;

/// Errori possibili durante l'invio di una richiesta.
#[derive(Debug, thiserror::Error)]
pub enum ErroreHttp {
    #[error("metodo HTTP non valido: {0}")]
    MetodoNonValido(String),
    #[error("errore di rete: {0}")]
    Rete(#[from] reqwest::Error),
}

/// Invia la richiesta e restituisce la risposta con le metriche raccolte.
pub async fn invia(richiesta: &Richiesta) -> Result<Risposta, ErroreHttp> {
    // Converte la stringa del metodo (es. "POST") nel tipo di reqwest.
    let metodo = reqwest::Method::from_bytes(richiesta.metodo.as_bytes())
        .map_err(|_| ErroreHttp::MetodoNonValido(richiesta.metodo.clone()))?;

    let client = reqwest::Client::new();
    let mut req = client.request(metodo, &richiesta.url);

    // Parametri della query string attivi (?chiave=valore).
    let query: Vec<(&str, &str)> = richiesta
        .params
        .iter()
        .filter(|p| p.attivo && !p.chiave.is_empty())
        .map(|p| (p.chiave.as_str(), p.valore.as_str()))
        .collect();
    if !query.is_empty() {
        req = req.query(&query);
    }

    // Aggiunge solo le intestazioni attive e con chiave non vuota.
    for h in &richiesta.headers {
        if h.attivo && !h.chiave.is_empty() {
            req = req.header(&h.chiave, &h.valore);
        }
    }

    // Autenticazione.
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
        _ => {}
    }

    // Aggiunge il corpo solo se presente.
    if !richiesta.body.is_empty() {
        req = req.body(richiesta.body.clone());
    }

    // Misura il tempo totale: dall'invio alla ricezione del corpo.
    let inizio = Instant::now();
    let resp = req.send().await?;
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
