//! Generazione di snippet di codice per la richiesta corrente in vari linguaggi
//! (oltre a cURL): JavaScript `fetch` e Python `requests`. Pratico per il
//! "Copia come…".

use crate::curl;
use crate::model::{Header, Richiesta};

/// Genera lo snippet per il linguaggio indicato ("curl" | "fetch" | "python").
pub fn genera(r: &Richiesta, linguaggio: &str) -> String {
    match linguaggio {
        "fetch" => fetch(r),
        "python" => python(r),
        _ => curl::genera(r),
    }
}

/// URL con i query param attivi accodati.
fn url_con_query(r: &Richiesta) -> String {
    let attivi: Vec<&Header> = r
        .params
        .iter()
        .filter(|p| p.attivo && !p.chiave.is_empty())
        .collect();
    if attivi.is_empty() {
        return r.url.clone();
    }
    let qs = attivi
        .iter()
        .map(|p| format!("{}={}", p.chiave, p.valore))
        .collect::<Vec<_>>()
        .join("&");
    let sep = if r.url.contains('?') { '&' } else { '?' };
    format!("{}{}{}", r.url, sep, qs)
}

/// Header attivi + eventuale Authorization da bearer/oauth2.
fn headers_effettivi(r: &Richiesta) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = r
        .headers
        .iter()
        .filter(|h| h.attivo && !h.chiave.is_empty())
        .map(|h| (h.chiave.clone(), h.valore.clone()))
        .collect();
    match r.auth.tipo.as_str() {
        "bearer" if !r.auth.token.is_empty() => {
            out.push(("Authorization".into(), format!("Bearer {}", r.auth.token)));
        }
        "oauth2" if !r.auth.oauth2.access_token.is_empty() => {
            out.push(("Authorization".into(), format!("Bearer {}", r.auth.oauth2.access_token)));
        }
        _ => {}
    }
    out
}

fn js_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn fetch(r: &Richiesta) -> String {
    let mut out = String::new();
    let headers = headers_effettivi(r);
    out.push_str(&format!("const res = await fetch({}, {{\n", js_str(&url_con_query(r))));
    out.push_str(&format!("  method: {},\n", js_str(&r.metodo)));
    if !headers.is_empty() {
        out.push_str("  headers: {\n");
        for (k, v) in &headers {
            out.push_str(&format!("    {}: {},\n", js_str(k), js_str(v)));
        }
        out.push_str("  },\n");
    }
    if !r.body.is_empty() && r.body_mode == "raw" {
        out.push_str(&format!("  body: {},\n", js_str(&r.body)));
    }
    out.push_str("});\nconst data = await res.json();\nconsole.log(data);");
    out
}

fn py_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn python(r: &Richiesta) -> String {
    let mut out = String::from("import requests\n\n");
    let headers = headers_effettivi(r);
    if !headers.is_empty() {
        out.push_str("headers = {\n");
        for (k, v) in &headers {
            out.push_str(&format!("    {}: {},\n", py_str(k), py_str(v)));
        }
        out.push_str("}\n");
    }
    let metodo = r.metodo.to_lowercase();
    out.push_str(&format!("resp = requests.{}({}", metodo, py_str(&url_con_query(r))));
    if !headers.is_empty() {
        out.push_str(", headers=headers");
    }
    if r.auth.tipo == "basic" {
        out.push_str(&format!(", auth=({}, {})", py_str(&r.auth.utente), py_str(&r.auth.password)));
    }
    if !r.body.is_empty() && r.body_mode == "raw" {
        out.push_str(&format!(", data={}", py_str(&r.body)));
    }
    out.push_str(")\nprint(resp.status_code)\nprint(resp.json())");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Auth;

    fn req() -> Richiesta {
        Richiesta {
            nome: "X".into(),
            metodo: "POST".into(),
            url: "https://api.test/login".into(),
            params: vec![Header { chiave: "lang".into(), valore: "it".into(), attivo: true }],
            headers: vec![Header { chiave: "Accept".into(), valore: "application/json".into(), attivo: true }],
            auth: Auth { tipo: "bearer".into(), token: "abc".into(), ..Auth::default() },
            body: "{\"u\":1}".into(),
            body_mode: "raw".into(),
            ..Default::default()
        }
    }

    #[test]
    fn fetch_snippet() {
        let s = genera(&req(), "fetch");
        assert!(s.contains("await fetch(\"https://api.test/login?lang=it\""));
        assert!(s.contains("method: \"POST\""));
        assert!(s.contains("\"Authorization\": \"Bearer abc\""));
        assert!(s.contains("body: \"{\\\"u\\\":1}\""));
    }

    #[test]
    fn python_snippet() {
        let s = genera(&req(), "python");
        assert!(s.contains("import requests"));
        assert!(s.contains("requests.post(\"https://api.test/login?lang=it\""));
        assert!(s.contains("headers=headers"));
        assert!(s.contains("data="));
    }

    #[test]
    fn curl_default() {
        assert!(genera(&req(), "curl").contains("curl"));
    }
}
