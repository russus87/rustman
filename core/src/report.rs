//! Generatore di report HTML di un'esecuzione (run) di richieste, per condividere
//! l'esito dei test funzionali/regressione fuori dalla CLI.

use crate::model::RisultatoRun;

/// Genera la pagina HTML del report a partire dagli esiti.
pub fn genera_html(esiti: &[RisultatoRun], titolo: &str) -> String {
    let mut tot_test = 0usize;
    let mut ko = 0usize;
    let mut errori = 0usize;
    let mut corpo = String::new();

    for e in esiti {
        let stato_req = if !e.errore.is_empty() {
            errori += 1;
            "ko"
        } else {
            "ok"
        };
        let metodo = e.metodo.to_lowercase();
        corpo.push_str(&format!(
            "<article class=\"req {stato_req}\"><div class=\"riga\">\
<span class=\"m {metodo}\">{}</span><span class=\"nome\">{}</span>\
<span class=\"meta\">{} · {}ms</span></div><code class=\"url\">{}</code>",
            esc(&e.metodo),
            esc(&e.nome),
            if e.errore.is_empty() { e.status.to_string() } else { "ERR".into() },
            e.tempo_ms,
            esc(&e.url),
        ));
        if !e.errore.is_empty() {
            corpo.push_str(&format!("<div class=\"errore\">{}</div>", esc(&e.errore)));
        }
        for t in &e.tests {
            tot_test += 1;
            let cls = if t.passato { "pass" } else { ko += 1; "fail" };
            corpo.push_str(&format!(
                "<div class=\"test {cls}\"><span class=\"esito\">{}</span> {} \
<span class=\"det\">{}</span></div>",
                if t.passato { "PASS" } else { "FAIL" },
                esc(&t.descrizione),
                esc(&t.dettaglio),
            ));
        }
        corpo.push_str("</article>");
    }

    let stato = if ko == 0 && errori == 0 { "verde" } else { "rosso" };
    let t = esc(titolo);
    let n = esiti.len();
    format!(
        "<!DOCTYPE html><html lang=\"it\"><head><meta charset=\"UTF-8\"/>\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\"/>\
<title>{t}</title><style>{STILE}</style></head><body>\
<header class=\"{stato}\"><h1>{t}</h1>\
<p>{n} richieste · {tot_test} test · {ko} falliti · {errori} errori</p></header>\
<main>{corpo}</main></body></html>"
    )
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

const STILE: &str = "\
*{box-sizing:border-box;margin:0;padding:0}\
body{font-family:-apple-system,Segoe UI,Roboto,sans-serif;background:#0e1117;color:#e8edf4;line-height:1.5}\
header{padding:22px 32px}header h1{font-size:1.4rem}header p{opacity:.85;font-size:.95rem;margin-top:4px}\
header.verde{background:linear-gradient(120deg,#1f7a35,#3fb950);color:#04140a}\
header.rosso{background:linear-gradient(120deg,#a01f1f,#f85149);color:#1a0404}\
main{padding:24px 32px;max-width:900px;margin:0 auto}\
.req{background:#161b22;border:1px solid #2a3340;border-left:3px solid #3fb950;border-radius:10px;padding:14px 16px;margin:12px 0}\
.req.ko{border-left-color:#f85149}\
.riga{display:flex;align-items:center;gap:10px}\
.m{font-family:monospace;font-weight:700;font-size:.78rem;padding:2px 8px;border-radius:6px;background:#0b0e14}\
.m.get{color:#3fb950}.m.post{color:#e2b340}.m.put,.m.patch{color:#4a9eff}.m.delete{color:#f85149}\
.nome{font-weight:600}.meta{margin-left:auto;color:#9aa7b8;font-size:.85rem;font-family:monospace}\
.url{display:block;color:#9aa7b8;font-family:monospace;font-size:.85rem;margin:6px 0;word-break:break-all}\
.errore{color:#f8918c;font-size:.85rem;margin:4px 0}\
.test{font-family:monospace;font-size:.85rem;padding:3px 0;display:flex;gap:8px;align-items:center}\
.esito{font-weight:700;font-size:.72rem;padding:1px 6px;border-radius:4px}\
.test.pass .esito{color:#56d364;background:rgba(63,185,80,.15)}\
.test.fail .esito{color:#f8918c;background:rgba(248,81,73,.15)}\
.det{margin-left:auto;color:#6e7b8a}";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RisultatoTest;

    #[test]
    fn genera_report() {
        let esiti = vec![RisultatoRun {
            nome: "Login".into(),
            metodo: "POST".into(),
            url: "https://x/login".into(),
            status: 200,
            status_text: "OK".into(),
            tempo_ms: 12,
            errore: String::new(),
            tests: vec![RisultatoTest {
                descrizione: "status == 200".into(),
                passato: true,
                dettaglio: "ok".into(),
            }],
        }];
        let html = genera_html(&esiti, "Report");
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Login"));
        assert!(html.contains("PASS"));
        assert!(html.contains("verde"));
    }
}
