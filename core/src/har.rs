//! Import da file HAR (HTTP Archive), l'export di rete dei browser/devtools.
//! Modulo puro: converte le `entries` del HAR in una collezione Rustman.

use crate::model::{Auth, CampoForm, EsportaCollezione, Header, NodoExport, Richiesta};
use serde::Deserialize;
use serde_json::Value;

/// Riconosce un file HAR (`log.entries`) e lo converte in collezione.
pub fn riconosci(contenuto: &str) -> Option<EsportaCollezione> {
    let v: Value = serde_json::from_str(contenuto).ok()?;
    // Deve avere log.entries come array.
    v.get("log")?.get("entries")?.as_array()?;
    let har: Har = serde_json::from_value(v).ok()?;
    Some(converti(har))
}

fn converti(har: Har) -> EsportaCollezione {
    let figli = har
        .log
        .entries
        .into_iter()
        .map(|e| NodoExport::Richiesta {
            richiesta: converti_richiesta(e.request),
        })
        .collect();
    EsportaCollezione {
        rustman: 1,
        nome: "Importata da HAR".to_string(),
        figli,
    }
}

fn converti_richiesta(req: HarReq) -> Richiesta {
    let (url, params_inline) = separa_query(&req.url);
    // Preferisci la queryString strutturata, se presente.
    let params = if req.query_string.is_empty() {
        params_inline
    } else {
        req.query_string
            .into_iter()
            .filter(|p| !p.name.is_empty())
            .map(|p| Header { chiave: p.name, valore: p.value, attivo: true })
            .collect()
    };

    // Header (esclusi gli pseudo-header HTTP/2 ":method", ":path", ...).
    let headers = req
        .headers
        .into_iter()
        .filter(|h| !h.name.is_empty() && !h.name.starts_with(':'))
        .map(|h| Header { chiave: h.name, valore: h.value, attivo: true })
        .collect();

    let (body, body_mode, form) = converti_body(req.post_data);
    let nome = nome_da_url(&url, &req.method);

    Richiesta {
        nome,
        metodo: if req.method.is_empty() { "GET".into() } else { req.method.to_uppercase() },
        url,
        headers,
        params,
        auth: Auth::default(),
        body,
        body_mode,
        form,
        tests: Vec::new(),
        pre_script: String::new(),
        post_script: String::new(),
    }
}

fn converti_body(post: Option<HarPost>) -> (String, String, Vec<CampoForm>) {
    let Some(p) = post else {
        return (String::new(), "raw".into(), Vec::new());
    };
    if p.mime.contains("x-www-form-urlencoded") && !p.params.is_empty() {
        let form = p
            .params
            .into_iter()
            .filter(|q| !q.name.is_empty())
            .map(|q| CampoForm {
                chiave: q.name,
                valore: q.value,
                tipo: "text".into(),
                file: String::new(),
                attivo: true,
            })
            .collect();
        (String::new(), "x-www-form-urlencoded".into(), form)
    } else {
        (p.text, "raw".into(), Vec::new())
    }
}

/// Spezza "http://x/y?a=1" in ("http://x/y", [a=1]).
fn separa_query(raw: &str) -> (String, Vec<Header>) {
    match raw.split_once('?') {
        None => (raw.to_string(), Vec::new()),
        Some((base, qs)) => {
            let params = qs
                .split('&')
                .filter(|p| !p.is_empty())
                .map(|p| {
                    let (k, v) = p.split_once('=').unwrap_or((p, ""));
                    Header { chiave: k.to_string(), valore: v.to_string(), attivo: true }
                })
                .collect();
            (base.to_string(), params)
        }
    }
}

/// Nome leggibile: "METODO /percorso".
fn nome_da_url(url: &str, metodo: &str) -> String {
    let percorso = url
        .split_once("://")
        .map(|(_, resto)| resto)
        .and_then(|r| r.split_once('/').map(|(_, p)| format!("/{p}")))
        .unwrap_or_else(|| url.to_string());
    format!("{} {}", metodo.to_uppercase(), percorso)
}

#[derive(Deserialize)]
struct Har {
    log: HarLog,
}
#[derive(Deserialize)]
struct HarLog {
    #[serde(default)]
    entries: Vec<HarEntry>,
}
#[derive(Deserialize)]
struct HarEntry {
    request: HarReq,
}
#[derive(Deserialize)]
struct HarReq {
    #[serde(default)]
    method: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    headers: Vec<NameVal>,
    #[serde(default, rename = "queryString")]
    query_string: Vec<NameVal>,
    #[serde(default, rename = "postData")]
    post_data: Option<HarPost>,
}
#[derive(Deserialize)]
struct NameVal {
    #[serde(default)]
    name: String,
    #[serde(default)]
    value: String,
}
#[derive(Deserialize)]
struct HarPost {
    #[serde(default, rename = "mimeType")]
    mime: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    params: Vec<NameVal>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const HAR: &str = r#"{
      "log": { "version": "1.2", "entries": [
        { "request": {
            "method": "POST",
            "url": "https://api.test/users?page=2",
            "headers": [ {"name":"Content-Type","value":"application/json"}, {"name":":authority","value":"x"} ],
            "queryString": [ {"name":"page","value":"2"} ],
            "postData": { "mimeType":"application/json", "text":"{\"n\":1}" }
        } }
      ] }
    }"#;

    #[test]
    fn importa_har() {
        let coll = riconosci(HAR).expect("HAR valido");
        assert_eq!(coll.nome, "Importata da HAR");
        let NodoExport::Richiesta { richiesta } = &coll.figli[0] else { panic!() };
        assert_eq!(richiesta.metodo, "POST");
        assert_eq!(richiesta.url, "https://api.test/users"); // query rimossa
        assert_eq!(richiesta.params[0].chiave, "page");
        assert_eq!(richiesta.body, "{\"n\":1}");
        // pseudo-header :authority scartato
        assert!(richiesta.headers.iter().all(|h| !h.chiave.starts_with(':')));
    }

    #[test]
    fn non_har_ignorato() {
        assert!(riconosci(r#"{"info":{},"item":[]}"#).is_none());
    }
}
