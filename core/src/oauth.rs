//! Ottenimento del token OAuth2 (grant `client_credentials` e `password`).
//!
//! Il grant `authorization_code` (con browser) è gestito dalla UI desktop;
//! qui copriamo i flussi server-to-server, usabili anche dalla CLI.
//! Il token ottenuto viene poi inviato come `Authorization: Bearer ...`.

use crate::http::ErroreHttp;
use crate::model::Oauth2;

/// Richiede un access token al `token_url` secondo il grant configurato.
pub async fn ottieni_token(cfg: &Oauth2) -> Result<String, ErroreHttp> {
    if cfg.token_url.trim().is_empty() {
        return Err(ErroreHttp::Oauth("manca il Token URL".into()));
    }

    // Parametri del corpo (application/x-www-form-urlencoded).
    let mut params: Vec<(&str, &str)> = Vec::new();
    let grant = if cfg.grant_type.is_empty() {
        "client_credentials"
    } else {
        cfg.grant_type.as_str()
    };
    params.push(("grant_type", grant));
    if !cfg.client_id.is_empty() {
        params.push(("client_id", &cfg.client_id));
    }
    if !cfg.client_secret.is_empty() {
        params.push(("client_secret", &cfg.client_secret));
    }
    if !cfg.scope.is_empty() {
        params.push(("scope", &cfg.scope));
    }
    if grant == "password" {
        params.push(("username", &cfg.username));
        params.push(("password", &cfg.password));
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(&cfg.token_url)
        .header("Accept", "application/json")
        .form(&params)
        .send()
        .await?;

    let status = resp.status();
    let testo = resp.text().await?;
    if !status.is_success() {
        return Err(ErroreHttp::Oauth(format!(
            "il server ha risposto {} — {}",
            status.as_u16(),
            testo
        )));
    }

    // La risposta è JSON con il campo "access_token".
    let json: serde_json::Value = serde_json::from_str(&testo)
        .map_err(|_| ErroreHttp::Oauth("risposta non in formato JSON".into()))?;
    json.get("access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ErroreHttp::Oauth("nessun access_token nella risposta".into()))
}
