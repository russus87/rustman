//! Sostituzione dei segnaposto `{{nome}}` con i valori di un ambiente.

use crate::model::{Auth, Header, Richiesta};
use std::collections::HashMap;

/// Sostituisce in `testo` ogni `{{nome}}` con il valore corrispondente.
/// Se una variabile non esiste, il segnaposto viene lasciato invariato.
pub fn sostituisci(testo: &str, vars: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(testo.len());
    let mut i = 0;
    while i < testo.len() {
        if testo[i..].starts_with("{{") {
            if let Some(fine) = testo[i + 2..].find("}}") {
                let nome = testo[i + 2..i + 2 + fine].trim();
                match vars.get(nome) {
                    Some(v) => out.push_str(v),
                    None => out.push_str(&testo[i..i + 2 + fine + 2]),
                }
                i = i + 2 + fine + 2;
                continue;
            }
        }
        // Carattere normale: lo copio così com'è (gestendo l'UTF-8).
        let ch = testo[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Restituisce una copia della richiesta con tutte le variabili risolte.
pub fn risolvi(r: &Richiesta, vars: &HashMap<String, String>) -> Richiesta {
    let s = |t: &str| sostituisci(t, vars);
    let mappa = |lista: &[Header]| -> Vec<Header> {
        lista
            .iter()
            .map(|h| Header {
                chiave: s(&h.chiave),
                valore: s(&h.valore),
                attivo: h.attivo,
            })
            .collect()
    };

    Richiesta {
        nome: r.nome.clone(),
        metodo: r.metodo.clone(),
        url: s(&r.url),
        headers: mappa(&r.headers),
        params: mappa(&r.params),
        auth: Auth {
            tipo: r.auth.tipo.clone(),
            token: s(&r.auth.token),
            utente: s(&r.auth.utente),
            password: s(&r.auth.password),
        },
        body: s(&r.body),
        tests: r.tests.clone(),
        pre_script: r.pre_script.clone(),
        post_script: r.post_script.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sostituisce_e_lascia_invariati_i_mancanti() {
        let mut v = HashMap::new();
        v.insert("base_url".to_string(), "https://api.x".to_string());
        assert_eq!(sostituisci("{{base_url}}/login", &v), "https://api.x/login");
        // variabile assente: resta il segnaposto
        assert_eq!(sostituisci("ciao {{nome}}", &v), "ciao {{nome}}");
        // spazi interni tollerati
        assert_eq!(sostituisci("{{ base_url }}", &v), "https://api.x");
    }
}
