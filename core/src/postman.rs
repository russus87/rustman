//! Import dal formato Postman (Collection v2.0/v2.1 ed Environment).
//!
//! Questo modulo è **puro**: non tocca il disco. Riconosce il tipo di file e
//! converte nei modelli interni di Rustman; la scrittura su file la fa `storage`.
//!
//! Cosa viene mappato:
//! - cartelle annidate (`item` senza `request`) → cartelle Rustman;
//! - richieste: metodo, URL, header, query params, body, auth, script;
//! - script `prerequest`/`test` (array `event`) → `pre_script`/`post_script`
//!   (l'API `pm.*` di Postman è già emulata dal frontend in `pm.js`);
//! - environment: `values` → variabili (`{{segnaposto}}` è la stessa sintassi).

use crate::model::{Auth, Environment, EsportaCollezione, Header, NodoExport, Richiesta, Variabile};
use serde::Deserialize;
use serde_json::Value;

/// Esito del riconoscimento: una collezione o un ambiente Postman.
pub enum ImportPostman {
    /// Una collezione e, se aveva variabili di collezione, l'ambiente derivato.
    Collezione(EsportaCollezione, Option<Environment>),
    Environment(Environment),
}

/// Riconosce e converte un JSON Postman. Restituisce `None` se non è Postman
/// (così il chiamante può ripiegare sul formato nativo Rustman).
pub fn riconosci(contenuto: &str) -> Option<ImportPostman> {
    let valore: Value = serde_json::from_str(contenuto).ok()?;

    if pare_collezione(&valore) {
        let coll: PmCollezione = serde_json::from_value(valore).ok()?;
        let (collezione, env) = converti_collezione(coll);
        Some(ImportPostman::Collezione(collezione, env))
    } else if pare_environment(&valore) {
        let env: PmEnvironment = serde_json::from_value(valore).ok()?;
        Some(ImportPostman::Environment(converti_environment(env)))
    } else {
        None
    }
}

/// Una collection Postman ha `info.schema` con "getpostman" e/o un array `item`.
fn pare_collezione(v: &Value) -> bool {
    let schema_ok = v
        .get("info")
        .and_then(|i| i.get("schema"))
        .and_then(|s| s.as_str())
        .map(|s| s.contains("getpostman"))
        .unwrap_or(false);
    schema_ok || v.get("item").map(|i| i.is_array()).unwrap_or(false)
}

/// Un environment Postman ha scope "environment" oppure `values` + `name`.
fn pare_environment(v: &Value) -> bool {
    let scope_ok = v
        .get("_postman_variable_scope")
        .and_then(|s| s.as_str())
        .map(|s| s == "environment")
        .unwrap_or(false);
    let values_ok =
        v.get("values").map(|x| x.is_array()).unwrap_or(false) && v.get("name").is_some();
    scope_ok || values_ok
}

// ===================== Strutture Postman (lette in modo tollerante) ==========

#[derive(Deserialize)]
struct PmCollezione {
    #[serde(default)]
    info: PmInfo,
    #[serde(default)]
    item: Vec<PmItem>,
    /// Variabili di collezione (diventano un ambiente Rustman).
    #[serde(default)]
    variable: Vec<PmEnvVar>,
    /// Script a livello di collezione (ereditati da tutte le richieste).
    #[serde(default)]
    event: Vec<PmEvent>,
}

#[derive(Deserialize, Default)]
struct PmInfo {
    #[serde(default)]
    name: String,
}

/// Un nodo Postman: cartella (ha `item`) o richiesta (ha `request`).
#[derive(Deserialize)]
struct PmItem {
    #[serde(default)]
    name: String,
    #[serde(default)]
    item: Option<Vec<PmItem>>,
    #[serde(default)]
    request: Option<PmRequest>,
    #[serde(default)]
    event: Vec<PmEvent>,
}

#[derive(Deserialize)]
struct PmRequest {
    #[serde(default)]
    method: String,
    #[serde(default)]
    url: Option<PmUrl>,
    #[serde(default)]
    header: Vec<PmHeader>,
    #[serde(default)]
    body: Option<PmBody>,
    #[serde(default)]
    auth: Option<PmAuth>,
}

/// L'URL Postman può essere una stringa o un oggetto strutturato.
#[derive(Deserialize)]
#[serde(untagged)]
enum PmUrl {
    Str(String),
    Obj(PmUrlObj),
}

#[derive(Deserialize)]
struct PmUrlObj {
    #[serde(default)]
    raw: String,
    #[serde(default)]
    query: Vec<PmQuery>,
}

#[derive(Deserialize)]
struct PmQuery {
    #[serde(default)]
    key: String,
    #[serde(default)]
    value: String,
    #[serde(default)]
    disabled: bool,
    /// Nei form-data un campo può essere "text" o "file": i file non sono
    /// trasferibili nell'export, quindi li scartiamo.
    #[serde(default, rename = "type")]
    tipo: String,
}

#[derive(Deserialize)]
struct PmHeader {
    #[serde(default)]
    key: String,
    #[serde(default)]
    value: String,
    #[serde(default)]
    disabled: bool,
}

#[derive(Deserialize)]
struct PmBody {
    #[serde(default)]
    mode: String,
    #[serde(default)]
    raw: String,
    #[serde(default)]
    urlencoded: Vec<PmQuery>,
    #[serde(default)]
    formdata: Vec<PmQuery>,
    #[serde(default)]
    graphql: Option<PmGraphql>,
}

#[derive(Deserialize)]
struct PmGraphql {
    #[serde(default)]
    query: String,
    #[serde(default)]
    variables: String,
}

/// Auth Postman: il tipo e una lista di coppie chiave/valore per quel tipo.
#[derive(Deserialize)]
struct PmAuth {
    #[serde(default, rename = "type")]
    tipo: String,
    #[serde(default)]
    bearer: Vec<PmKv>,
    #[serde(default)]
    basic: Vec<PmKv>,
    #[serde(default)]
    apikey: Vec<PmKv>,
}

#[derive(Deserialize)]
struct PmKv {
    #[serde(default)]
    key: String,
    #[serde(default)]
    value: String,
}

#[derive(Deserialize)]
struct PmEvent {
    #[serde(default)]
    listen: String,
    #[serde(default)]
    script: PmScript,
}

#[derive(Deserialize, Default)]
struct PmScript {
    /// Le righe dello script (Postman le salva come array di stringhe).
    #[serde(default)]
    exec: Vec<String>,
}

#[derive(Deserialize)]
struct PmEnvironment {
    #[serde(default)]
    name: String,
    #[serde(default)]
    values: Vec<PmEnvVar>,
}

#[derive(Deserialize)]
struct PmEnvVar {
    #[serde(default)]
    key: String,
    #[serde(default)]
    value: String,
    #[serde(default = "vero")]
    enabled: bool,
}

fn vero() -> bool {
    true
}

// ============================ Conversioni ====================================

fn converti_collezione(c: PmCollezione) -> (EsportaCollezione, Option<Environment>) {
    let nome = nome_o_default(c.info.name, "Importata da Postman");

    // Gli script a livello di collezione sono ereditati da tutte le richieste.
    let (pre_coll, post_coll) = converti_eventi(c.event);
    let figli = c
        .item
        .into_iter()
        .map(|i| converti_item(i, &pre_coll, &post_coll))
        .collect();

    // Le variabili di collezione diventano un ambiente dedicato.
    let env = converti_variabili(format!("{nome} (variabili)"), c.variable);

    let collezione = EsportaCollezione {
        rustman: 1,
        nome,
        figli,
    };
    (collezione, env)
}

/// Converte un nodo Postman in un nodo dell'albero Rustman, propagando gli
/// script ereditati (`pre_ered`/`post_ered`) verso le richieste annidate.
fn converti_item(mut item: PmItem, pre_ered: &str, post_ered: &str) -> NodoExport {
    // Prendiamo gli eventi del nodo (lasciando `item` utilizzabile più sotto).
    let (pre_self, post_self) = converti_eventi(std::mem::take(&mut item.event));
    let pre = concatena_script(pre_ered, &pre_self);
    let post = concatena_script(post_ered, &post_self);

    // Se ha `item` è una cartella; altrimenti è una richiesta.
    if let Some(figli) = item.item {
        NodoExport::Cartella {
            nome: nome_o_default(item.name, "Cartella"),
            figli: figli
                .into_iter()
                .map(|f| converti_item(f, &pre, &post))
                .collect(),
        }
    } else {
        NodoExport::Richiesta {
            richiesta: converti_richiesta(item, pre, post),
        }
    }
}

fn converti_richiesta(item: PmItem, pre_script: String, post_script: String) -> Richiesta {
    let nome = nome_o_default(item.name, "Richiesta");
    let req = item.request.unwrap_or(PmRequest {
        method: "GET".into(),
        url: None,
        header: Vec::new(),
        body: None,
        auth: None,
    });

    let (url, mut params) = converti_url(req.url);

    let mut headers: Vec<Header> = req
        .header
        .into_iter()
        .filter(|h| !h.key.is_empty())
        .map(|h| Header {
            chiave: h.key,
            valore: h.value,
            attivo: !h.disabled,
        })
        .collect();

    let auth = converti_auth(req.auth, &mut headers, &mut params);
    let body = converti_body(req.body, &mut headers);

    Richiesta {
        nome,
        metodo: if req.method.is_empty() {
            "GET".into()
        } else {
            req.method.to_uppercase()
        },
        url,
        headers,
        params,
        auth,
        body,
        body_mode: "raw".into(),
        form: Vec::new(),
        tests: Vec::new(),
        pre_script,
        post_script,
        impostazioni: Default::default(),
        tags: Vec::new(),
        descrizione: String::new(),
        esempi: Vec::new(),
    }
}

/// Restituisce (url_senza_query, params). La query finisce nei params di
/// Rustman, che l'`http` riaccoda da solo: evitiamo così i duplicati.
fn converti_url(url: Option<PmUrl>) -> (String, Vec<Header>) {
    match url {
        None => (String::new(), Vec::new()),
        Some(PmUrl::Str(s)) => separa_query(&s),
        Some(PmUrl::Obj(o)) => {
            let (base, params_inline) = separa_query(&o.raw);
            // Preferisci l'array `query` strutturato se presente.
            if o.query.is_empty() {
                (base, params_inline)
            } else {
                let params = o
                    .query
                    .into_iter()
                    .filter(|q| !q.key.is_empty())
                    .map(|q| Header {
                        chiave: q.key,
                        valore: q.value,
                        attivo: !q.disabled,
                    })
                    .collect();
                (base, params)
            }
        }
    }
}

/// Spezza "http://x/y?a=1&b=2" in ("http://x/y", [a=1, b=2]).
fn separa_query(raw: &str) -> (String, Vec<Header>) {
    match raw.split_once('?') {
        None => (raw.to_string(), Vec::new()),
        Some((base, qs)) => {
            let params = qs
                .split('&')
                .filter(|p| !p.is_empty())
                .map(|p| {
                    let (k, v) = p.split_once('=').unwrap_or((p, ""));
                    Header {
                        chiave: k.to_string(),
                        valore: v.to_string(),
                        attivo: true,
                    }
                })
                .collect();
            (base.to_string(), params)
        }
    }
}

fn converti_auth(
    auth: Option<PmAuth>,
    headers: &mut Vec<Header>,
    params: &mut Vec<Header>,
) -> Auth {
    let Some(a) = auth else {
        return Auth::default();
    };
    match a.tipo.as_str() {
        "bearer" => Auth {
            tipo: "bearer".into(),
            token: kv(&a.bearer, "token"),
            ..Auth::default()
        },
        "basic" => Auth {
            tipo: "basic".into(),
            utente: kv(&a.basic, "username"),
            password: kv(&a.basic, "password"),
            ..Auth::default()
        },
        // Rustman non ha un tipo "apikey": la trasformiamo in header o param.
        "apikey" => {
            let chiave = kv(&a.apikey, "key");
            let valore = kv(&a.apikey, "value");
            if !chiave.is_empty() {
                let voce = Header {
                    chiave,
                    valore,
                    attivo: true,
                };
                if kv(&a.apikey, "in") == "query" {
                    params.push(voce);
                } else {
                    headers.push(voce);
                }
            }
            Auth::default()
        }
        _ => Auth::default(),
    }
}

/// Cerca il valore di una chiave nella lista di coppie di un blocco auth.
fn kv(lista: &[PmKv], chiave: &str) -> String {
    lista
        .iter()
        .find(|kv| kv.key == chiave)
        .map(|kv| kv.value.clone())
        .unwrap_or_default()
}

fn converti_body(body: Option<PmBody>, headers: &mut Vec<Header>) -> String {
    let Some(b) = body else {
        return String::new();
    };
    match b.mode.as_str() {
        "raw" => b.raw,
        "urlencoded" => {
            assicura_content_type(headers, "application/x-www-form-urlencoded");
            coppie_a_querystring(&b.urlencoded)
        }
        "formdata" => {
            // I file non sono trasferibili: riportiamo solo i campi testo.
            coppie_a_querystring(&b.formdata)
        }
        "graphql" => {
            assicura_content_type(headers, "application/json");
            let g = b.graphql.unwrap_or(PmGraphql {
                query: String::new(),
                variables: String::new(),
            });
            // Le variabili sono già una stringa JSON; se vuote usiamo {}.
            let vars = if g.variables.trim().is_empty() {
                "{}".to_string()
            } else {
                g.variables
            };
            let payload = serde_json::json!({ "query": g.query });
            // Componiamo a mano per non riserializzare le variabili.
            format!(
                "{{\"query\":{},\"variables\":{}}}",
                payload["query"], vars
            )
        }
        _ => String::new(),
    }
}

fn assicura_content_type(headers: &mut Vec<Header>, valore: &str) {
    let presente = headers
        .iter()
        .any(|h| h.chiave.eq_ignore_ascii_case("content-type"));
    if !presente {
        headers.push(Header {
            chiave: "Content-Type".into(),
            valore: valore.into(),
            attivo: true,
        });
    }
}

fn coppie_a_querystring(coppie: &[PmQuery]) -> String {
    coppie
        .iter()
        // Scarta i campi file (binari non trasferibili) e quelli disabilitati.
        .filter(|q| !q.key.is_empty() && !q.disabled && q.tipo != "file")
        .map(|q| format!("{}={}", q.key, q.value))
        .collect::<Vec<_>>()
        .join("&")
}

/// Unisce uno script ereditato con quello proprio, separati da una riga vuota.
fn concatena_script(ereditato: &str, proprio: &str) -> String {
    match (ereditato.trim().is_empty(), proprio.trim().is_empty()) {
        (true, _) => proprio.to_string(),
        (_, true) => ereditato.to_string(),
        _ => format!("{ereditato}\n\n{proprio}"),
    }
}

/// Estrae gli script pre/post dagli eventi (`prerequest` e `test`).
fn converti_eventi(eventi: Vec<PmEvent>) -> (String, String) {
    let mut pre = String::new();
    let mut post = String::new();
    for e in eventi {
        let codice = e.script.exec.join("\n");
        match e.listen.as_str() {
            "prerequest" => pre = codice,
            "test" => post = codice,
            _ => {}
        }
    }
    (pre, post)
}

fn converti_environment(e: PmEnvironment) -> Environment {
    converti_variabili(nome_o_default(e.name, "Environment Postman"), e.values)
        .unwrap_or(Environment {
            nome: "Environment Postman".into(),
            variabili: Vec::new(),
        })
}

/// Costruisce un ambiente dalle variabili (scartando quelle disabilitate).
/// Restituisce `None` se non c'è alcuna variabile valida.
fn converti_variabili(nome: String, values: Vec<PmEnvVar>) -> Option<Environment> {
    let variabili: Vec<Variabile> = values
        .into_iter()
        .filter(|v| v.enabled && !v.key.is_empty())
        .map(|v| Variabile {
            chiave: v.key,
            valore: v.value,
            segreto: false,
        })
        .collect();
    if variabili.is_empty() {
        None
    } else {
        Some(Environment { nome, variabili })
    }
}

fn nome_o_default(nome: String, default: &str) -> String {
    if nome.trim().is_empty() {
        default.to_string()
    } else {
        nome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLLEZIONE: &str = r#"{
      "info": { "name": "Demo API", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json" },
      "variable": [ { "key": "base_url", "value": "https://api.test" } ],
      "event": [ { "listen": "prerequest", "script": { "exec": ["console.log('coll')"] } } ],
      "item": [
        {
          "name": "Auth",
          "event": [ { "listen": "test", "script": { "exec": ["// test di cartella"] } } ],
          "item": [
            {
              "name": "Login",
              "event": [
                { "listen": "prerequest", "script": { "exec": ["pm.environment.set('t', 1)"] } },
                { "listen": "test", "script": { "exec": ["pm.test('ok', () => pm.response.to.have.status(200))"] } }
              ],
              "request": {
                "method": "POST",
                "header": [ { "key": "X-Trace", "value": "1" } ],
                "auth": { "type": "bearer", "bearer": [ { "key": "token", "value": "abc" } ] },
                "body": { "mode": "raw", "raw": "{\"u\":\"a\"}" },
                "url": { "raw": "https://api.test/login?lang=it", "query": [ { "key": "lang", "value": "it" } ] }
              }
            }
          ]
        }
      ]
    }"#;

    const ENVIRONMENT: &str = r#"{
      "name": "Staging",
      "_postman_variable_scope": "environment",
      "values": [
        { "key": "base_url", "value": "https://staging.test", "enabled": true },
        { "key": "segreto", "value": "x", "enabled": false }
      ]
    }"#;

    #[test]
    fn importa_collezione_postman() {
        let Some(ImportPostman::Collezione(c, env)) = riconosci(COLLEZIONE) else {
            panic!("non riconosciuta come collezione");
        };
        assert_eq!(c.nome, "Demo API");
        let NodoExport::Cartella { nome, figli } = &c.figli[0] else {
            panic!("atteso una cartella");
        };
        assert_eq!(nome, "Auth");
        let NodoExport::Richiesta { richiesta } = &figli[0] else {
            panic!("attesa una richiesta");
        };
        assert_eq!(richiesta.nome, "Login");
        assert_eq!(richiesta.metodo, "POST");
        assert_eq!(richiesta.url, "https://api.test/login"); // query rimossa dall'url
        assert_eq!(richiesta.params.len(), 1);
        assert_eq!(richiesta.params[0].chiave, "lang");
        assert_eq!(richiesta.auth.tipo, "bearer");
        assert_eq!(richiesta.auth.token, "abc");
        assert_eq!(richiesta.body, "{\"u\":\"a\"}");

        // Script ereditati: pre della collezione + pre della richiesta;
        // test della cartella + test della richiesta.
        assert!(richiesta.pre_script.contains("console.log('coll')"));
        assert!(richiesta.pre_script.contains("pm.environment.set"));
        assert!(richiesta.post_script.contains("test di cartella"));
        assert!(richiesta.post_script.contains("pm.test"));

        // Le variabili di collezione diventano un ambiente.
        let env = env.expect("atteso un ambiente dalle variabili di collezione");
        assert_eq!(env.nome, "Demo API (variabili)");
        assert_eq!(env.variabili[0].chiave, "base_url");
    }

    #[test]
    fn formdata_scarta_i_file() {
        let json = r#"{"item":[{"name":"Up","request":{"method":"POST","url":"https://x.test/u",
          "body":{"mode":"formdata","formdata":[
            {"key":"nome","value":"mario","type":"text"},
            {"key":"avatar","type":"file","src":"/tmp/a.png"}
          ]}}}]}"#;
        let Some(ImportPostman::Collezione(c, _)) = riconosci(json) else {
            panic!();
        };
        let NodoExport::Richiesta { richiesta } = &c.figli[0] else {
            panic!();
        };
        // Solo il campo testo, il file è scartato.
        assert_eq!(richiesta.body, "nome=mario");
    }

    #[test]
    fn importa_environment_postman() {
        let Some(ImportPostman::Environment(e)) = riconosci(ENVIRONMENT) else {
            panic!("non riconosciuto come environment");
        };
        assert_eq!(e.nome, "Staging");
        // La variabile disabilitata viene scartata.
        assert_eq!(e.variabili.len(), 1);
        assert_eq!(e.variabili[0].chiave, "base_url");
    }

    #[test]
    fn json_non_postman_non_riconosciuto() {
        assert!(riconosci(r#"{"rustman":1,"nome":"X","figli":[]}"#).is_none());
        assert!(riconosci("non json").is_none());
    }

    #[test]
    fn url_come_stringa_semplice() {
        let json = r#"{"item":[{"name":"G","request":{"method":"GET","url":"https://x.test/a?p=1&q=2"}}]}"#;
        let Some(ImportPostman::Collezione(c, _)) = riconosci(json) else {
            panic!();
        };
        let NodoExport::Richiesta { richiesta } = &c.figli[0] else {
            panic!();
        };
        assert_eq!(richiesta.url, "https://x.test/a");
        assert_eq!(richiesta.params.len(), 2);
    }
}
