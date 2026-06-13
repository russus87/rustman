//! Snapshot / golden testing: confronta il corpo di una risposta con una
//! baseline registrata, ignorando i campi volatili indicati (ignore-paths).
//!
//! Gli ignore-path usano la notazione a punti con `*` come jolly di segmento:
//! - `data.timestamp` → ignora quel campo;
//! - `items.*.id` → ignora `id` in ogni elemento dell'array `items`;
//! - `*.updatedAt` → ignora `updatedAt` in ogni proprietà di primo livello.

use serde_json::Value;

/// Confronta `attuale` con la `baseline`. Restituisce (uguale, dettaglio).
/// Se entrambi sono JSON, confronta dopo aver rimosso gli ignore-path.
pub fn confronta(baseline: &str, attuale: &str, ignora: &[String]) -> (bool, String) {
    match (
        serde_json::from_str::<Value>(baseline),
        serde_json::from_str::<Value>(attuale),
    ) {
        (Ok(mut a), Ok(mut b)) => {
            for path in ignora {
                let segs: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
                rimuovi(&mut a, &segs);
                rimuovi(&mut b, &segs);
            }
            if a == b {
                (true, "uguale alla baseline".into())
            } else {
                (false, "la risposta differisce dalla baseline (snapshot)".into())
            }
        }
        // Corpi non-JSON: confronto testuale grezzo.
        _ => {
            let uguale = baseline.trim() == attuale.trim();
            (
                uguale,
                if uguale { "uguale alla baseline".into() } else { "differisce dalla baseline".into() },
            )
        }
    }
}

/// Rimuove ricorsivamente dal valore JSON il percorso indicato (con jolly `*`).
fn rimuovi(v: &mut Value, segs: &[&str]) {
    let Some((head, rest)) = segs.split_first() else {
        return;
    };
    match v {
        Value::Object(map) => {
            if *head == "*" {
                if rest.is_empty() {
                    map.clear();
                } else {
                    for sub in map.values_mut() {
                        rimuovi(sub, rest);
                    }
                }
            } else if rest.is_empty() {
                map.remove(*head);
            } else if let Some(sub) = map.get_mut(*head) {
                rimuovi(sub, rest);
            }
        }
        Value::Array(arr) => {
            if *head == "*" {
                if rest.is_empty() {
                    arr.clear();
                } else {
                    for sub in arr.iter_mut() {
                        rimuovi(sub, rest);
                    }
                }
            } else if let Ok(i) = head.parse::<usize>() {
                if rest.is_empty() {
                    if i < arr.len() {
                        arr.remove(i);
                    }
                } else if let Some(sub) = arr.get_mut(i) {
                    rimuovi(sub, rest);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uguale_ignorando_campi_volatili() {
        let base = r#"{"id":1,"nome":"Mario","ts":100,"items":[{"id":9,"v":"a"}]}"#;
        let now = r#"{"id":1,"nome":"Mario","ts":999,"items":[{"id":42,"v":"a"}]}"#;
        let ignora = vec!["ts".to_string(), "items.*.id".to_string()];
        let (ok, _) = confronta(base, now, &ignora);
        assert!(ok);
    }

    #[test]
    fn rileva_differenza_reale() {
        let base = r#"{"nome":"Mario"}"#;
        let now = r#"{"nome":"Luigi"}"#;
        let (ok, _) = confronta(base, now, &[]);
        assert!(!ok);
    }
}
