//! Security scan delle risposte: controlli passivi su header e cookie, utili
//! per testing autorizzato (header di sicurezza mancanti, CORS troppo permissivi,
//! info leak, cookie non sicuri).

use crate::model::{Risposta, SecurityAvviso};

/// Analizza la risposta e restituisce gli avvisi di sicurezza.
pub fn analizza(risposta: &Risposta) -> Vec<SecurityAvviso> {
    let mut out = Vec::new();
    let trova = |nome: &str| {
        risposta
            .headers
            .iter()
            .find(|h| h.chiave.eq_ignore_ascii_case(nome))
            .map(|h| h.valore.clone())
    };
    let mut avviso = |livello: &str, titolo: &str, dettaglio: &str| {
        out.push(SecurityAvviso {
            livello: livello.into(),
            titolo: titolo.into(),
            dettaglio: dettaglio.into(),
        });
    };

    // Header di sicurezza mancanti.
    if trova("strict-transport-security").is_none() {
        avviso("medio", "HSTS assente", "Manca `Strict-Transport-Security`: il browser potrebbe usare HTTP.");
    }
    if trova("content-security-policy").is_none() {
        avviso("medio", "CSP assente", "Manca `Content-Security-Policy`: difesa ridotta contro XSS/injection.");
    }
    if trova("x-content-type-options").is_none() {
        avviso("info", "Niente nosniff", "Manca `X-Content-Type-Options: nosniff`.");
    }
    if trova("x-frame-options").is_none()
        && !trova("content-security-policy").map(|c| c.contains("frame-ancestors")).unwrap_or(false)
    {
        avviso("medio", "Clickjacking", "Manca `X-Frame-Options` o `frame-ancestors` in CSP.");
    }

    // CORS troppo permissivo.
    if trova("access-control-allow-origin").as_deref() == Some("*") {
        let creds = trova("access-control-allow-credentials").as_deref() == Some("true");
        let livello = if creds { "alto" } else { "medio" };
        avviso(livello, "CORS aperto", "`Access-Control-Allow-Origin: *` espone l'API a qualsiasi origine.");
    }

    // Information disclosure.
    if let Some(s) = trova("server") {
        if s.chars().any(|c| c.is_ascii_digit()) {
            avviso("info", "Server header", &format!("`Server: {s}` rivela il software/versione."));
        }
    }
    if let Some(p) = trova("x-powered-by") {
        avviso("info", "X-Powered-By", &format!("`X-Powered-By: {p}` rivela la tecnologia."));
    }

    // Cookie non sicuri.
    for h in risposta.headers.iter().filter(|h| h.chiave.eq_ignore_ascii_case("set-cookie")) {
        let v = h.valore.to_lowercase();
        let nome = h.valore.split('=').next().unwrap_or("cookie");
        if !v.contains("secure") {
            avviso("medio", "Cookie senza Secure", &format!("Il cookie `{nome}` non ha l'attributo Secure."));
        }
        if !v.contains("httponly") {
            avviso("info", "Cookie senza HttpOnly", &format!("Il cookie `{nome}` è accessibile da JavaScript."));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Header;

    fn risp(headers: Vec<(&str, &str)>) -> Risposta {
        Risposta {
            status: 200,
            status_text: "OK".into(),
            headers: headers.into_iter().map(|(k, v)| Header { chiave: k.into(), valore: v.into(), attivo: true }).collect(),
            body: String::new(),
            tempo_ms: 1,
            dimensione: 0,
        }
    }

    #[test]
    fn rileva_problemi() {
        let r = risp(vec![
            ("Access-Control-Allow-Origin", "*"),
            ("Server", "nginx/1.25.1"),
            ("Set-Cookie", "sid=abc; Path=/"),
        ]);
        let a = analizza(&r);
        assert!(a.iter().any(|x| x.titolo == "CORS aperto"));
        assert!(a.iter().any(|x| x.titolo == "Server header"));
        assert!(a.iter().any(|x| x.titolo == "Cookie senza Secure"));
        assert!(a.iter().any(|x| x.titolo == "HSTS assente"));
    }

    #[test]
    fn risposta_blindata_pochi_avvisi() {
        let r = risp(vec![
            ("Strict-Transport-Security", "max-age=31536000"),
            ("Content-Security-Policy", "default-src 'self'; frame-ancestors 'none'"),
            ("X-Content-Type-Options", "nosniff"),
        ]);
        let a = analizza(&r);
        assert!(a.iter().all(|x| x.titolo != "HSTS assente" && x.titolo != "CSP assente"));
    }
}
