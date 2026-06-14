//! Rustman core: logica riutilizzabile da desktop e (in futuro) dal web.

pub mod codegen;
pub mod curl;
pub mod doc;
pub mod git;
pub mod har;
pub mod http;
pub mod jsonschema;
pub mod model;
pub mod oauth;
pub mod openapi;
pub mod perf;
pub mod postman;
pub mod report;
pub mod security;
pub mod snapshot;
pub mod storage;
pub mod test;
pub mod textdiff;
pub mod vars;

#[cfg(test)]
mod tests {
    use super::model::{Auth, Header, Richiesta};

    use super::http;

    /// Test d'integrazione reale (richiede rete): esegue una GET e controlla
    /// che la risposta abbia status 200. Escluso di default con #[ignore];
    /// si lancia con: `cargo test -p rustman-core -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn invia_get_reale() {
        let r = Richiesta {
            nome: "test".into(),
            metodo: "GET".into(),
            url: "https://httpbin.org/get".into(),
            headers: vec![],
            params: vec![],
            auth: Auth::default(),
            body: String::new(),
            body_mode: "raw".into(),
            form: vec![],
            tests: vec![],
            pre_script: String::new(),
            post_script: String::new(),
            impostazioni: Default::default(),
        tags: Vec::new(),
        descrizione: String::new(),
        };
        let resp = http::invia(&r).await.expect("la richiesta deve riuscire");
        assert_eq!(resp.status, 200);
        assert!(resp.tempo_ms < 30_000);
        assert!(!resp.body.is_empty());
    }

    /// La richiesta deve serializzarsi/deserializzarsi correttamente in JSON.
    #[test]
    fn richiesta_roundtrip_json() {
        let r = Richiesta {
            nome: "test".into(),
            metodo: "GET".into(),
            url: "https://example.com".into(),
            headers: vec![Header {
                chiave: "Accept".into(),
                valore: "application/json".into(),
                attivo: true,
            }],
            params: vec![],
            auth: Auth::default(),
            body: String::new(),
            body_mode: "raw".into(),
            form: vec![],
            tests: vec![],
            pre_script: String::new(),
            post_script: String::new(),
            impostazioni: Default::default(),
        tags: Vec::new(),
        descrizione: String::new(),
        };
        let json = serde_json::to_string(&r).unwrap();
        let r2: Richiesta = serde_json::from_str(&json).unwrap();
        assert_eq!(r2.metodo, "GET");
        assert_eq!(r2.headers.len(), 1);
    }
}
