//! Valutatore JSONPath (sottoinsieme pragmatico) per le asserzioni.
//! Supporta: `$`, `.chiave`, `['chiave']`, `[indice]`, `[*]` (tutti gli elementi
//! di un array o valori di un oggetto) e `..chiave` (discesa ricorsiva).
//! I filtri (`[?(...)]`) non sono supportati.

use serde_json::Value;

enum Passo {
    Chiave(String),
    Indice(usize),
    Tutti,
    Ricorsivo(String),
}

/// Estrae tutti i valori che corrispondono all'espressione JSONPath.
pub fn estrai(radice: &Value, espressione: &str) -> Vec<Value> {
    let passi = analizza(espressione);
    let mut correnti: Vec<&Value> = vec![radice];
    for passo in &passi {
        let mut prossimi: Vec<&Value> = Vec::new();
        for v in &correnti {
            match passo {
                Passo::Chiave(k) => {
                    if let Some(x) = v.get(k) {
                        prossimi.push(x);
                    }
                }
                Passo::Indice(i) => {
                    if let Some(x) = v.get(i) {
                        prossimi.push(x);
                    }
                }
                Passo::Tutti => match v {
                    Value::Array(a) => prossimi.extend(a.iter()),
                    Value::Object(m) => prossimi.extend(m.values()),
                    _ => {}
                },
                Passo::Ricorsivo(k) => raccogli_ricorsivo(v, k, &mut prossimi),
            }
        }
        correnti = prossimi;
    }
    correnti.into_iter().cloned().collect()
}

/// Trova ricorsivamente tutti i valori sotto la chiave `k` in qualsiasi punto.
fn raccogli_ricorsivo<'a>(v: &'a Value, k: &str, out: &mut Vec<&'a Value>) {
    match v {
        Value::Object(m) => {
            for (key, val) in m {
                if key == k {
                    out.push(val);
                }
                raccogli_ricorsivo(val, k, out);
            }
        }
        Value::Array(a) => {
            for el in a {
                raccogli_ricorsivo(el, k, out);
            }
        }
        _ => {}
    }
}

/// Analizza l'espressione in una sequenza di passi.
fn analizza(espressione: &str) -> Vec<Passo> {
    let mut passi = Vec::new();
    let s = espressione.trim().trim_start_matches('$');
    let mut chars = s.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            '.' => {
                chars.next();
                if chars.peek() == Some(&'.') {
                    chars.next();
                    let nome = leggi_nome(&mut chars);
                    if !nome.is_empty() {
                        passi.push(Passo::Ricorsivo(nome));
                    }
                } else if chars.peek() == Some(&'*') {
                    chars.next();
                    passi.push(Passo::Tutti);
                } else {
                    let nome = leggi_nome(&mut chars);
                    if !nome.is_empty() {
                        passi.push(Passo::Chiave(nome));
                    }
                }
            }
            '[' => {
                chars.next();
                let mut dentro = String::new();
                for x in chars.by_ref() {
                    if x == ']' {
                        break;
                    }
                    dentro.push(x);
                }
                let dentro = dentro.trim().trim_matches(['\'', '"']);
                if dentro == "*" {
                    passi.push(Passo::Tutti);
                } else if let Ok(i) = dentro.parse::<usize>() {
                    passi.push(Passo::Indice(i));
                } else if !dentro.is_empty() {
                    passi.push(Passo::Chiave(dentro.to_string()));
                }
            }
            _ => {
                // Nome senza punto iniziale (es. "data.id").
                let nome = leggi_nome(&mut chars);
                if nome.is_empty() {
                    chars.next();
                } else {
                    passi.push(Passo::Chiave(nome));
                }
            }
        }
    }
    passi
}

fn leggi_nome(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut nome = String::new();
    while let Some(&c) = chars.peek() {
        if c == '.' || c == '[' || c == '*' {
            break;
        }
        nome.push(c);
        chars.next();
    }
    nome
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn percorso_e_wildcard() {
        let v = json!({ "data": { "items": [ {"id": 1}, {"id": 2} ] } });
        // indice
        assert_eq!(estrai(&v, "data.items[0].id"), vec![json!(1)]);
        // wildcard su array
        assert_eq!(estrai(&v, "data.items[*].id"), vec![json!(1), json!(2)]);
    }

    #[test]
    fn discesa_ricorsiva() {
        let v = json!({ "a": { "id": 1, "b": { "id": 2 } }, "c": [ { "id": 3 } ] });
        let mut r = estrai(&v, "$..id");
        r.sort_by_key(|x| x.as_i64().unwrap());
        assert_eq!(r, vec![json!(1), json!(2), json!(3)]);
    }
}
