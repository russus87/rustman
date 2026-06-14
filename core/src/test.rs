//! Motore di test: valuta le asserzioni di una richiesta sulla risposta ricevuta.

use crate::model::{Asserzione, RisultatoTest, Risposta};
use serde_json::Value;

/// Valuta tutte le asserzioni attive e restituisce un esito per ciascuna.
pub fn valuta(asserzioni: &[Asserzione], risposta: &Risposta) -> Vec<RisultatoTest> {
    asserzioni
        .iter()
        // "snapshot" è gestito a parte (serve l'accesso alla baseline su file).
        .filter(|a| a.attivo && a.tipo != "snapshot")
        .map(|a| valuta_una(a, risposta))
        .collect()
}

/// Valuta una singola asserzione.
fn valuta_una(a: &Asserzione, risposta: &Risposta) -> RisultatoTest {
    // "schema": contract testing del body JSON contro uno schema (in `atteso`).
    if a.tipo == "schema" {
        return valida_schema(a, risposta);
    }
    // "jsonpath": valuta un'espressione JSONPath sul body (passa se almeno un
    // match soddisfa l'operatore).
    if a.tipo == "jsonpath" {
        return valuta_jsonpath(a, risposta);
    }

    // Ricava il valore "ottenuto" in base al tipo di asserzione.
    let ottenuto: Option<String> = match a.tipo.as_str() {
        "status" => Some(risposta.status.to_string()),
        "tempo" => Some(risposta.tempo_ms.to_string()),
        "header" => risposta
            .headers
            .iter()
            .find(|h| h.chiave.eq_ignore_ascii_case(&a.campo))
            .map(|h| h.valore.clone()),
        "body" => Some(risposta.body.clone()),
        "json" => valore_json(&risposta.body, &a.campo),
        _ => None,
    };

    let descrizione = descrivi(a);

    let Some(ottenuto) = ottenuto else {
        return RisultatoTest {
            descrizione,
            passato: false,
            dettaglio: if a.tipo == "json" {
                format!("campo '{}' non trovato nel JSON", a.campo)
            } else if a.tipo == "header" {
                format!("header '{}' assente", a.campo)
            } else {
                "valore non disponibile".to_string()
            },
        };
    };

    let passato = confronta(&ottenuto, &a.operatore, &a.atteso);
    RisultatoTest {
        descrizione,
        passato,
        dettaglio: format!("ottenuto: {}", abbrevia(&ottenuto)),
    }
}

/// Valida il body JSON della risposta contro lo schema JSON in `a.atteso`.
fn valida_schema(a: &Asserzione, risposta: &Risposta) -> RisultatoTest {
    let descrizione = "body conforme allo schema".to_string();
    let schema: Value = match serde_json::from_str(&a.atteso) {
        Ok(v) => v,
        Err(e) => {
            return RisultatoTest {
                descrizione,
                passato: false,
                dettaglio: format!("schema non valido: {e}"),
            }
        }
    };
    let dato: Value = match serde_json::from_str(&risposta.body) {
        Ok(v) => v,
        Err(_) => {
            return RisultatoTest {
                descrizione,
                passato: false,
                dettaglio: "la risposta non è JSON".to_string(),
            }
        }
    };
    let errori = crate::jsonschema::valida(&schema, &dato);
    if errori.is_empty() {
        RisultatoTest {
            descrizione,
            passato: true,
            dettaglio: "valido".to_string(),
        }
    } else {
        RisultatoTest {
            descrizione,
            passato: false,
            dettaglio: abbrevia(&errori.join("; ")),
        }
    }
}

/// Valuta un'asserzione JSONPath: estrae i match e verifica se almeno uno
/// soddisfa l'operatore rispetto al valore atteso.
fn valuta_jsonpath(a: &Asserzione, risposta: &Risposta) -> RisultatoTest {
    let descrizione = format!("jsonpath '{}' {} {}", a.campo, a.operatore, a.atteso);
    let radice: Value = match serde_json::from_str(&risposta.body) {
        Ok(v) => v,
        Err(_) => {
            return RisultatoTest { descrizione, passato: false, dettaglio: "la risposta non è JSON".into() }
        }
    };
    let matches = crate::jsonpath::estrai(&radice, &a.campo);
    if matches.is_empty() {
        return RisultatoTest { descrizione, passato: false, dettaglio: "nessun match".into() };
    }
    let come_str = |v: &Value| match v {
        Value::String(s) => s.clone(),
        altro => altro.to_string(),
    };
    let passato = matches.iter().any(|m| confronta(&come_str(m), &a.operatore, &a.atteso));
    let dettaglio = format!("{} match · es. {}", matches.len(), abbrevia(&come_str(&matches[0])));
    RisultatoTest { descrizione, passato, dettaglio }
}

/// Applica l'operatore tra valore ottenuto e atteso.
fn confronta(ottenuto: &str, operatore: &str, atteso: &str) -> bool {
    match operatore {
        "==" => ottenuto.trim() == atteso.trim(),
        "!=" => ottenuto.trim() != atteso.trim(),
        "contiene" => ottenuto.contains(atteso),
        "<" | ">" => {
            // Confronto numerico: servono due numeri validi.
            match (ottenuto.trim().parse::<f64>(), atteso.trim().parse::<f64>()) {
                (Ok(o), Ok(e)) => {
                    if operatore == "<" {
                        o < e
                    } else {
                        o > e
                    }
                }
                _ => false,
            }
        }
        _ => false,
    }
}

/// Naviga un body JSON seguendo un path con punti (es. "data.items.0.id").
fn valore_json(body: &str, path: &str) -> Option<String> {
    let radice: Value = serde_json::from_str(body).ok()?;
    let mut corrente = &radice;
    for parte in path.split('.') {
        if parte.is_empty() {
            continue;
        }
        corrente = match corrente {
            Value::Object(map) => map.get(parte)?,
            Value::Array(arr) => arr.get(parte.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(match corrente {
        Value::String(s) => s.clone(),
        altro => altro.to_string(),
    })
}

/// Descrizione leggibile dell'asserzione.
fn descrivi(a: &Asserzione) -> String {
    match a.tipo.as_str() {
        "status" => format!("status {} {}", a.operatore, a.atteso),
        "tempo" => format!("tempo {} {} ms", a.operatore, a.atteso),
        "header" => format!("header '{}' {} {}", a.campo, a.operatore, a.atteso),
        "body" => format!("body {} {}", a.operatore, a.atteso),
        "json" => format!("json '{}' {} {}", a.campo, a.operatore, a.atteso),
        altro => format!("{} {} {}", altro, a.operatore, a.atteso),
    }
}

/// Tronca i valori lunghi per il dettaglio mostrato in UI.
fn abbrevia(s: &str) -> String {
    let s = s.trim();
    if s.chars().count() > 60 {
        format!("{}…", s.chars().take(60).collect::<String>())
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Header;

    fn risposta_finta() -> Risposta {
        Risposta {
            status: 200,
            status_text: "OK".into(),
            headers: vec![Header {
                chiave: "Content-Type".into(),
                valore: "application/json".into(),
                attivo: true,
            }],
            body: r#"{"data":{"id":42,"nome":"Mario"}}"#.into(),
            tempo_ms: 120,
            dimensione: 33,
        }
    }

    fn ass(tipo: &str, op: &str, campo: &str, atteso: &str) -> Asserzione {
        Asserzione {
            tipo: tipo.into(),
            operatore: op.into(),
            campo: campo.into(),
            atteso: atteso.into(),
            attivo: true,
        }
    }

    #[test]
    fn asserzioni_varie() {
        let r = risposta_finta();
        let asserzioni = vec![
            ass("status", "==", "", "200"),         // pass
            ass("tempo", "<", "", "500"),           // pass
            ass("header", "contiene", "Content-Type", "json"), // pass
            ass("json", "==", "data.id", "42"),     // pass
            ass("json", "==", "data.nome", "Luigi"), // fail
            ass("json", "==", "data.assente", "x"), // fail (mancante)
        ];
        let esiti = valuta(&asserzioni, &r);
        let passati: Vec<bool> = esiti.iter().map(|e| e.passato).collect();
        assert_eq!(passati, vec![true, true, true, true, false, false]);
    }
}
