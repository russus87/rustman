//! Generazione e parsing di comandi `curl` a partire da una `Richiesta`.
//! Permette il "Copia come cURL" e l'import incollando un comando curl.

use crate::model::{Auth, CampoForm, Header, Richiesta};

/// Genera un comando `curl` leggibile (con continuazioni di riga) per la richiesta.
pub fn genera(r: &Richiesta) -> String {
    let mut parti: Vec<String> = vec!["curl".into(), format!("-X {}", r.metodo)];
    parti.push(quota(&url_con_query(r)));

    for h in &r.headers {
        if h.attivo && !h.chiave.is_empty() {
            parti.push(format!("-H {}", quota(&format!("{}: {}", h.chiave, h.valore))));
        }
    }

    match r.auth.tipo.as_str() {
        "bearer" if !r.auth.token.is_empty() => {
            parti.push(format!("-H {}", quota(&format!("Authorization: Bearer {}", r.auth.token))));
        }
        "basic" => {
            parti.push(format!("-u {}", quota(&format!("{}:{}", r.auth.utente, r.auth.password))));
        }
        "oauth2" if !r.auth.oauth2.access_token.is_empty() => {
            parti.push(format!(
                "-H {}",
                quota(&format!("Authorization: Bearer {}", r.auth.oauth2.access_token))
            ));
        }
        _ => {}
    }

    match r.body_mode.as_str() {
        "form-data" => {
            for c in &r.form {
                if !c.attivo || c.chiave.is_empty() {
                    continue;
                }
                let campo = if c.tipo == "file" {
                    format!("{}=@{}", c.chiave, c.file)
                } else {
                    format!("{}={}", c.chiave, c.valore)
                };
                parti.push(format!("-F {}", quota(&campo)));
            }
        }
        "x-www-form-urlencoded" => {
            let dati = campi_urlencoded(&r.form);
            if !dati.is_empty() {
                parti.push(format!("--data {}", quota(&dati)));
            }
        }
        _ => {
            if !r.body.is_empty() {
                parti.push(format!("--data-raw {}", quota(&r.body)));
            }
        }
    }

    parti.join(" \\\n  ")
}

/// Analizza un comando `curl` e ne ricava una `Richiesta` (None se non valido).
pub fn analizza(comando: &str) -> Option<Richiesta> {
    let tok = tokenizza(comando);
    let mut i = if tok.first().map(|s| s.as_str()) == Some("curl") { 1 } else { 0 };

    let mut metodo: Option<String> = None;
    let mut url = String::new();
    let mut headers: Vec<Header> = Vec::new();
    let mut auth = Auth::default();
    let mut form: Vec<CampoForm> = Vec::new();
    let mut datas: Vec<String> = Vec::new();
    let mut form_data = false;

    while i < tok.len() {
        let t = tok[i].as_str();
        match t {
            "-X" | "--request" => {
                i += 1;
                metodo = tok.get(i).cloned();
            }
            "-H" | "--header" => {
                i += 1;
                if let Some(h) = tok.get(i) {
                    if let Some((k, v)) = h.split_once(':') {
                        let (k, v) = (k.trim(), v.trim());
                        if let Some(token) = v.strip_prefix("Bearer ") {
                            if k.eq_ignore_ascii_case("authorization") {
                                auth.tipo = "bearer".into();
                                auth.token = token.trim().to_string();
                                i += 1;
                                continue;
                            }
                        }
                        headers.push(Header {
                            chiave: k.to_string(),
                            valore: v.to_string(),
                            attivo: true,
                        });
                    }
                }
            }
            "-u" | "--user" => {
                i += 1;
                if let Some(u) = tok.get(i) {
                    let (utente, password) = u.split_once(':').unwrap_or((u.as_str(), ""));
                    auth.tipo = "basic".into();
                    auth.utente = utente.to_string();
                    auth.password = password.to_string();
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-ascii" | "--data-binary" => {
                i += 1;
                if let Some(d) = tok.get(i) {
                    datas.push(d.clone());
                }
            }
            "-F" | "--form" => {
                i += 1;
                form_data = true;
                if let Some(f) = tok.get(i) {
                    if let Some((k, v)) = f.split_once('=') {
                        if let Some(path) = v.strip_prefix('@') {
                            form.push(CampoForm {
                                chiave: k.to_string(),
                                valore: String::new(),
                                tipo: "file".into(),
                                file: path.to_string(),
                                attivo: true,
                            });
                        } else {
                            form.push(CampoForm {
                                chiave: k.to_string(),
                                valore: v.to_string(),
                                tipo: "text".into(),
                                file: String::new(),
                                attivo: true,
                            });
                        }
                    }
                }
            }
            "--url" => {
                i += 1;
                if let Some(u) = tok.get(i) {
                    url = u.clone();
                }
            }
            "-A" | "--user-agent" => {
                i += 1;
                if let Some(a) = tok.get(i) {
                    headers.push(Header {
                        chiave: "User-Agent".into(),
                        valore: a.clone(),
                        attivo: true,
                    });
                }
            }
            // Altri flag con valore che ignoriamo (saltando anche il valore).
            "-e" | "--referer" | "--connect-timeout" | "-m" | "--max-time" | "-o" | "--output" => {
                i += 1;
            }
            // Flag booleani noti: nessun valore da saltare.
            s if s.starts_with('-') => {}
            // Token "nudo": è l'URL.
            s => {
                if url.is_empty() {
                    url = s.to_string();
                }
            }
        }
        i += 1;
    }

    if url.is_empty() {
        return None;
    }

    let body_mode = if form_data {
        "form-data"
    } else if !datas.is_empty() {
        "x-www-form-urlencoded"
    } else {
        "raw"
    };
    let body = if form_data { String::new() } else { datas.join("&") };
    let metodo = metodo.unwrap_or_else(|| {
        if form_data || !body.is_empty() {
            "POST".into()
        } else {
            "GET".into()
        }
    });

    Some(Richiesta {
        nome: "Importata da cURL".into(),
        metodo: metodo.to_uppercase(),
        url,
        headers,
        params: Vec::new(),
        auth,
        body,
        body_mode: body_mode.into(),
        form,
        tests: Vec::new(),
        pre_script: String::new(),
        post_script: String::new(),
        impostazioni: Default::default(),
    })
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

fn campi_urlencoded(form: &[CampoForm]) -> String {
    form.iter()
        .filter(|c| c.attivo && !c.chiave.is_empty() && c.tipo != "file")
        .map(|c| format!("{}={}", c.chiave, c.valore))
        .collect::<Vec<_>>()
        .join("&")
}

/// Racchiude un valore tra apici singoli per la shell (escape di `'`).
fn quota(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Spezza una stringa in token rispettando apici singoli/doppi, escape e
/// le continuazioni di riga (`\` a fine riga).
fn tokenizza(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_tok = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                } else if let Some(n) = chars.next() {
                    cur.push(n);
                    in_tok = true;
                }
            }
            '\'' => {
                in_tok = true;
                for q in chars.by_ref() {
                    if q == '\'' {
                        break;
                    }
                    cur.push(q);
                }
            }
            '"' => {
                in_tok = true;
                while let Some(q) = chars.next() {
                    if q == '"' {
                        break;
                    }
                    if q == '\\' {
                        if let Some(n) = chars.next() {
                            cur.push(n);
                        }
                    } else {
                        cur.push(q);
                    }
                }
            }
            c if c.is_whitespace() => {
                if in_tok {
                    out.push(std::mem::take(&mut cur));
                    in_tok = false;
                }
            }
            c => {
                cur.push(c);
                in_tok = true;
            }
        }
    }
    if in_tok {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genera_curl_di_base() {
        let mut r = Richiesta {
            nome: "X".into(),
            metodo: "POST".into(),
            url: "https://api.test/login".into(),
            headers: vec![Header { chiave: "Accept".into(), valore: "application/json".into(), attivo: true }],
            params: vec![Header { chiave: "lang".into(), valore: "it".into(), attivo: true }],
            auth: Auth { tipo: "bearer".into(), token: "abc".into(), ..Auth::default() },
            body: "{\"u\":\"a\"}".into(),
            body_mode: "raw".into(),
            form: vec![],
            tests: vec![],
            pre_script: String::new(),
            post_script: String::new(),
            impostazioni: Default::default(),
        };
        r.metodo = "POST".into();
        let cmd = genera(&r);
        assert!(cmd.contains("curl"));
        assert!(cmd.contains("-X POST"));
        assert!(cmd.contains("https://api.test/login?lang=it"));
        assert!(cmd.contains("'Authorization: Bearer abc'"));
        assert!(cmd.contains("--data-raw"));
    }

    #[test]
    fn analizza_curl_completo() {
        let cmd = "curl -X POST 'https://api.test/u?x=1' \\\n  -H 'Content-Type: application/json' \\\n  -H \"Authorization: Bearer tok123\" \\\n  --data-raw '{\"a\":1}'";
        let r = analizza(cmd).expect("curl valido");
        assert_eq!(r.metodo, "POST");
        assert_eq!(r.url, "https://api.test/u?x=1");
        assert_eq!(r.auth.tipo, "bearer");
        assert_eq!(r.auth.token, "tok123");
        assert!(r.headers.iter().any(|h| h.chiave == "Content-Type"));
        assert_eq!(r.body, "{\"a\":1}");
    }

    #[test]
    fn analizza_form_e_basic() {
        let cmd = "curl https://x.test/up -u mario:segreto -F campo=ciao -F file=@/tmp/a.png";
        let r = analizza(cmd).unwrap();
        assert_eq!(r.metodo, "POST");
        assert_eq!(r.auth.tipo, "basic");
        assert_eq!(r.auth.utente, "mario");
        assert_eq!(r.body_mode, "form-data");
        assert_eq!(r.form.len(), 2);
        assert_eq!(r.form[1].tipo, "file");
        assert_eq!(r.form[1].file, "/tmp/a.png");
    }

    #[test]
    fn roundtrip_get_semplice() {
        let cmd = genera(&analizza("curl https://x.test/a").unwrap());
        let r = analizza(&cmd).unwrap();
        assert_eq!(r.metodo, "GET");
        assert_eq!(r.url, "https://x.test/a");
    }
}
