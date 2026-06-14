//! Validatore JSON Schema minimale (sottoinsieme), per il "contract testing":
//! verifica che una risposta JSON rispetti uno schema (es. quello dichiarato in
//! uno spec OpenAPI). Supporta: `type`, `required`, `properties`, `items`,
//! `enum`, `nullable` (stile OpenAPI). Non risolve i `$ref` (gli schemi vanno
//! già espansi; l'import OpenAPI lo fa).

use serde_json::{json, Value};

/// Inferisce un JSON Schema (bozza) da un valore JSON di esempio.
pub fn inferisci(v: &Value) -> Value {
    match v {
        Value::Null => json!({ "type": "string", "nullable": true }),
        Value::Bool(_) => json!({ "type": "boolean" }),
        Value::Number(n) => {
            json!({ "type": if n.is_i64() || n.is_u64() { "integer" } else { "number" } })
        }
        Value::String(_) => json!({ "type": "string" }),
        Value::Array(a) => {
            let items = a.first().map(inferisci).unwrap_or_else(|| json!({}));
            json!({ "type": "array", "items": items })
        }
        Value::Object(m) => {
            let mut props = serde_json::Map::new();
            let mut required = Vec::new();
            for (k, val) in m {
                props.insert(k.clone(), inferisci(val));
                required.push(k.clone());
            }
            json!({ "type": "object", "properties": props, "required": required })
        }
    }
}

/// Valida `dato` contro `schema`. Restituisce la lista degli errori (vuota = ok).
/// `percorso` è usato internamente per messaggi tipo "data.items[0].id".
pub fn valida(schema: &Value, dato: &Value) -> Vec<String> {
    let mut errori = Vec::new();
    valida_in(schema, dato, "$", &mut errori);
    errori
}

fn valida_in(schema: &Value, dato: &Value, percorso: &str, errori: &mut Vec<String>) {
    let Some(obj) = schema.as_object() else {
        return; // schema non oggetto: niente da verificare
    };

    // nullable (OpenAPI): se il dato è null ed è ammesso, ok.
    if dato.is_null() {
        if obj.get("nullable").and_then(Value::as_bool).unwrap_or(false) {
            return;
        }
    }

    // type
    if let Some(tipo) = obj.get("type").and_then(Value::as_str) {
        if !tipo_corrisponde(tipo, dato) {
            errori.push(format!(
                "{percorso}: atteso tipo '{tipo}', trovato '{}'",
                tipo_di(dato)
            ));
            return; // inutile controllare oltre se il tipo è sbagliato
        }
    }

    // enum
    if let Some(vals) = obj.get("enum").and_then(Value::as_array) {
        if !vals.contains(dato) {
            errori.push(format!("{percorso}: valore non nell'enum"));
        }
    }

    // object: required + properties
    if let Some(props) = obj.get("properties").and_then(Value::as_object) {
        if let Some(map) = dato.as_object() {
            if let Some(req) = obj.get("required").and_then(Value::as_array) {
                for r in req.iter().filter_map(Value::as_str) {
                    if !map.contains_key(r) {
                        errori.push(format!("{percorso}.{r}: campo obbligatorio assente"));
                    }
                }
            }
            for (chiave, sub_schema) in props {
                if let Some(sub_dato) = map.get(chiave) {
                    valida_in(sub_schema, sub_dato, &format!("{percorso}.{chiave}"), errori);
                }
            }
        }
    }

    // array: items
    if let Some(items) = obj.get("items") {
        if let Some(arr) = dato.as_array() {
            for (i, el) in arr.iter().enumerate() {
                valida_in(items, el, &format!("{percorso}[{i}]"), errori);
            }
        }
    }
}

/// Vero se il `dato` è del tipo JSON Schema indicato.
fn tipo_corrisponde(tipo: &str, dato: &Value) -> bool {
    match tipo {
        "object" => dato.is_object(),
        "array" => dato.is_array(),
        "string" => dato.is_string(),
        "boolean" => dato.is_boolean(),
        "integer" => dato.is_i64() || dato.is_u64(),
        "number" => dato.is_number(),
        "null" => dato.is_null(),
        _ => true, // tipo sconosciuto: non blocchiamo
    }
}

fn tipo_di(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn valida_oggetto_conforme() {
        let schema = json!({
            "type": "object",
            "required": ["id", "nome"],
            "properties": {
                "id": { "type": "integer" },
                "nome": { "type": "string" },
                "tags": { "type": "array", "items": { "type": "string" } }
            }
        });
        let ok = json!({ "id": 1, "nome": "Mario", "tags": ["a", "b"] });
        assert!(valida(&schema, &ok).is_empty());
    }

    #[test]
    fn rileva_violazioni() {
        let schema = json!({
            "type": "object",
            "required": ["id"],
            "properties": { "id": { "type": "integer" }, "ruolo": { "enum": ["admin", "user"] } }
        });
        // id mancante + tipo errato su ruolo dentro enum
        let ko = json!({ "ruolo": "altro" });
        let errori = valida(&schema, &ko);
        assert_eq!(errori.len(), 2);
        assert!(errori.iter().any(|e| e.contains("id")));
        assert!(errori.iter().any(|e| e.contains("enum")));
    }

    #[test]
    fn nullable_ammette_null() {
        let schema = json!({ "type": "string", "nullable": true });
        assert!(valida(&schema, &Value::Null).is_empty());
    }

    #[test]
    fn inferisci_e_valida_roundtrip() {
        let dato = json!({ "id": 1, "nome": "Mario", "tags": ["a", "b"] });
        let schema = inferisci(&dato);
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["id"]["type"], "integer");
        assert_eq!(schema["properties"]["tags"]["type"], "array");
        // Lo schema inferito valida il dato originale.
        assert!(valida(&schema, &dato).is_empty());
    }
}
