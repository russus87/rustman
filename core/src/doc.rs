//! Generatore di documentazione HTML statica a partire dall'albero delle
//! collezioni. Produce una singola pagina autonoma (CSS inline), navigabile,
//! con metodo, URL, header, parametri, auth, body e asserzioni di ogni richiesta.

use crate::model::{Albero, Auth, Header, Nodo, Richiesta};

/// Genera la pagina HTML di documentazione per l'intero workspace.
pub fn genera(collezioni: &Albero) -> String {
    let mut corpo = String::new();
    let mut indice = String::new();

    for coll in collezioni {
        let id = ancora(&coll.nome);
        indice.push_str(&format!(
            "<li><a href=\"#{id}\">{}</a></li>",
            esc(&coll.nome)
        ));
        corpo.push_str(&format!(
            "<section class=\"coll\"><h2 id=\"{id}\">{}</h2>",
            esc(&coll.nome)
        ));
        rendi_nodi(&coll.figli, &mut corpo);
        corpo.push_str("</section>");
    }

    if collezioni.is_empty() {
        corpo.push_str("<p class=\"vuoto\">Nessuna collezione.</p>");
    }

    format!(
        "<!DOCTYPE html><html lang=\"it\"><head><meta charset=\"UTF-8\"/>\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\"/>\
<title>Rustman — Documentazione API</title><style>{STILE}</style></head><body>\
<header><h1>📘 Documentazione API</h1><p>Generata con Rustman</p></header>\
<nav><ul>{indice}</ul></nav><main>{corpo}</main></body></html>"
    )
}

/// Renderizza ricorsivamente i nodi (cartelle e richieste).
fn rendi_nodi(figli: &[Nodo], out: &mut String) {
    for n in figli {
        match n {
            Nodo::Cartella { nome, figli, .. } => {
                out.push_str(&format!("<div class=\"cartella\"><h3>📁 {}</h3>", esc(nome)));
                rendi_nodi(figli, out);
                out.push_str("</div>");
            }
            Nodo::Richiesta { richiesta, .. } => rendi_richiesta(richiesta, out),
        }
    }
}

fn rendi_richiesta(r: &Richiesta, out: &mut String) {
    let metodo = r.metodo.to_lowercase();
    out.push_str(&format!(
        "<article class=\"req\"><div class=\"riga\"><span class=\"m {metodo}\">{}</span>\
<span class=\"nome\">{}</span></div><code class=\"url\">{}</code>",
        esc(&r.metodo),
        esc(&r.nome),
        esc(&r.url)
    ));

    rendi_tabella(out, "Parametri", &r.params);
    rendi_tabella(out, "Header", &r.headers);
    rendi_auth(out, &r.auth);

    if !r.body.is_empty() {
        out.push_str(&format!(
            "<h4>Body ({})</h4><pre>{}</pre>",
            esc(&r.body_mode),
            esc(&r.body)
        ));
    }
    if !r.tests.is_empty() {
        out.push_str("<h4>Asserzioni</h4><ul class=\"tests\">");
        for t in &r.tests {
            let desc = if t.tipo == "schema" {
                "body conforme allo schema".to_string()
            } else if t.campo.is_empty() {
                format!("{} {} {}", t.tipo, t.operatore, t.atteso)
            } else {
                format!("{} '{}' {} {}", t.tipo, t.campo, t.operatore, t.atteso)
            };
            out.push_str(&format!("<li>{}</li>", esc(&desc)));
        }
        out.push_str("</ul>");
    }
    out.push_str("</article>");
}

fn rendi_tabella(out: &mut String, titolo: &str, voci: &[Header]) {
    let attive: Vec<&Header> = voci.iter().filter(|h| !h.chiave.is_empty()).collect();
    if attive.is_empty() {
        return;
    }
    out.push_str(&format!("<h4>{titolo}</h4><table>"));
    for h in attive {
        out.push_str(&format!(
            "<tr><td class=\"k\">{}</td><td>{}</td></tr>",
            esc(&h.chiave),
            esc(&h.valore)
        ));
    }
    out.push_str("</table>");
}

fn rendi_auth(out: &mut String, auth: &Auth) {
    let testo = match auth.tipo.as_str() {
        "bearer" => "Bearer Token".to_string(),
        "basic" => format!("Basic — utente: {}", esc(&auth.utente)),
        "oauth2" => format!("OAuth 2.0 ({})", esc(&auth.oauth2.grant_type)),
        _ => return,
    };
    out.push_str(&format!("<h4>Auth</h4><p class=\"auth\">{testo}</p>"));
}

/// Genera un'ancora HTML sicura da un nome.
fn ancora(nome: &str) -> String {
    nome.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect()
}

/// Escape minimale per HTML.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

const STILE: &str = "\
*{box-sizing:border-box;margin:0;padding:0}\
body{font-family:-apple-system,Segoe UI,Roboto,sans-serif;background:#0e1117;color:#e8edf4;line-height:1.6;display:grid;grid-template-columns:240px 1fr;min-height:100vh}\
header{grid-column:1/-1;padding:24px 32px;background:linear-gradient(120deg,#f74c00,#dea584);color:#1a1000}\
header h1{font-size:1.6rem}header p{opacity:.85;font-size:.9rem}\
nav{border-right:1px solid #2a3340;padding:20px 16px;position:sticky;top:0;align-self:start;max-height:100vh;overflow:auto}\
nav ul{list-style:none}nav a{color:#dea584;text-decoration:none;display:block;padding:6px 8px;border-radius:6px;font-size:.92rem}\
nav a:hover{background:#1b2230;color:#fff}\
main{padding:28px 36px;max-width:900px}\
.coll>h2{font-size:1.4rem;margin:28px 0 12px;color:#ff7a3c;border-bottom:1px solid #2a3340;padding-bottom:6px}\
.cartella{margin:14px 0 14px 8px;padding-left:14px;border-left:2px solid #2a3340}\
.cartella h3{font-size:1.05rem;color:#9aa7b8;margin:12px 0 8px}\
.req{background:#161b22;border:1px solid #2a3340;border-radius:12px;padding:16px 18px;margin:12px 0}\
.riga{display:flex;align-items:center;gap:10px;margin-bottom:6px}\
.m{font-family:monospace;font-weight:700;font-size:.8rem;padding:2px 8px;border-radius:6px;background:#0b0e14}\
.m.get{color:#3fb950}.m.post{color:#e2b340}.m.put,.m.patch{color:#4a9eff}.m.delete{color:#f85149}\
.nome{font-weight:600}\
.url{display:block;color:#9aa7b8;font-family:monospace;font-size:.88rem;word-break:break-all;margin-bottom:8px}\
h4{font-size:.78rem;text-transform:uppercase;letter-spacing:.05em;color:#6e7b8a;margin:12px 0 4px}\
table{width:100%;border-collapse:collapse;font-size:.88rem}\
td{padding:3px 8px;border-bottom:1px solid #20262f;vertical-align:top}\
td.k{color:#dea584;font-family:monospace;width:30%}\
pre{background:#0b0e14;border:1px solid #2a3340;border-radius:8px;padding:12px;overflow:auto;font-size:.85rem}\
.tests{list-style:none;font-family:monospace;font-size:.85rem}.tests li{padding:2px 0;color:#9aa7b8}\
.auth{font-size:.9rem;color:#9aa7b8}.vuoto{color:#6e7b8a}\
@media(max-width:720px){body{grid-template-columns:1fr}nav{position:static;max-height:none}}";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Collezione;

    #[test]
    fn genera_html_di_base() {
        let r = Richiesta {
            nome: "Login".into(),
            metodo: "POST".into(),
            url: "https://api.test/login".into(),
            headers: vec![Header { chiave: "Accept".into(), valore: "application/json".into(), attivo: true }],
            params: vec![],
            auth: Auth { tipo: "bearer".into(), ..Auth::default() },
            body: "{\"u\":1}".into(),
            body_mode: "raw".into(),
            form: vec![],
            tests: vec![],
            pre_script: String::new(),
            post_script: String::new(),
        };
        let albero = vec![Collezione {
            nome: "Auth API".into(),
            dir: "auth-api".into(),
            figli: vec![Nodo::Richiesta { nome: "Login".into(), file: "auth-api/login.json".into(), richiesta: r }],
        }];
        let html = genera(&albero);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Auth API"));
        assert!(html.contains("Login"));
        assert!(html.contains("https://api.test/login"));
        assert!(html.contains("Bearer"));
    }
}
