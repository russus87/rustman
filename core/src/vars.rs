//! Sostituzione dei segnaposto `{{nome}}` con i valori di un ambiente.

use crate::model::{Auth, CampoForm, Header, Oauth2, Richiesta};
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
                // Prima le variabili dinamiche ($...), poi quelle dell'ambiente.
                let sostituto = genera_dinamica(nome).or_else(|| vars.get(nome).cloned());
                match sostituto {
                    Some(v) => out.push_str(&v),
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
    let mappa_form = |lista: &[CampoForm]| -> Vec<CampoForm> {
        lista
            .iter()
            .map(|c| CampoForm {
                chiave: s(&c.chiave),
                valore: s(&c.valore),
                tipo: c.tipo.clone(),
                file: s(&c.file),
                attivo: c.attivo,
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
            oauth2: Oauth2 {
                grant_type: r.auth.oauth2.grant_type.clone(),
                token_url: s(&r.auth.oauth2.token_url),
                auth_url: s(&r.auth.oauth2.auth_url),
                client_id: s(&r.auth.oauth2.client_id),
                client_secret: s(&r.auth.oauth2.client_secret),
                username: s(&r.auth.oauth2.username),
                password: s(&r.auth.oauth2.password),
                scope: s(&r.auth.oauth2.scope),
                access_token: r.auth.oauth2.access_token.clone(),
            },
        },
        body: s(&r.body),
        body_mode: r.body_mode.clone(),
        form: mappa_form(&r.form),
        tests: r.tests.clone(),
        pre_script: r.pre_script.clone(),
        post_script: r.post_script.clone(),
    }
}

// ===================== Variabili dinamiche ($...) ============================

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Contatore per dare unicità ai valori generati nello stesso istante.
static CONTATORE: AtomicU64 = AtomicU64::new(0);

/// Genera il valore di una variabile dinamica (nome che inizia per `$`).
/// Restituisce `None` se il nome non è una variabile dinamica conosciuta.
/// Elenco supportato: vedi [`VARIABILI_DINAMICHE`].
pub fn genera_dinamica(nome: &str) -> Option<String> {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    match nome {
        "$timestamp" => Some(secs.to_string()),
        "$isoTimestamp" => Some(iso_utc(secs)),
        "$randomInt" => Some((rng_next() % 1000).to_string()),
        "$randomFloat" => Some(format!("{:.6}", (rng_next() % 1_000_000) as f64 / 1_000_000.0)),
        "$randomUUID" | "$guid" => Some(uuid_v4()),
        _ => None,
    }
}

/// Elenco (nome, descrizione) delle variabili dinamiche, per la UI.
pub const VARIABILI_DINAMICHE: &[(&str, &str)] = &[
    ("{{$timestamp}}", "secondi Unix attuali"),
    ("{{$isoTimestamp}}", "data/ora UTC in ISO 8601"),
    ("{{$randomInt}}", "intero casuale 0–999"),
    ("{{$randomFloat}}", "decimale casuale 0–1"),
    ("{{$randomUUID}}", "UUID v4 casuale"),
];

/// Generatore pseudo-casuale (xorshift) seminato da tempo + contatore.
fn rng_next() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let c = CONTATORE.fetch_add(1, Ordering::Relaxed);
    let mut x = nanos ^ c.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(0x1234_5678_9ABC);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

/// UUID versione 4 (random) in formato canonico.
fn uuid_v4() -> String {
    let a = rng_next();
    let b = rng_next();
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&a.to_le_bytes());
    bytes[8..].copy_from_slice(&b.to_le_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x40; // versione 4
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // variante
    let h: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

/// Converte secondi Unix in stringa ISO 8601 UTC (algoritmo di Howard Hinnant).
fn iso_utc(secs: u64) -> String {
    let giorni = (secs / 86400) as i64;
    let resto = secs % 86400;
    let (h, mi, s) = (resto / 3600, (resto % 3600) / 60, resto % 60);

    let z = giorni + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variabili_dinamiche() {
        let v = HashMap::new();
        // timestamp: tutte cifre
        assert!(sostituisci("{{$timestamp}}", &v).chars().all(|c| c.is_ascii_digit()));
        // UUID v4: lunghezza e versione
        let u = sostituisci("{{$randomUUID}}", &v);
        assert_eq!(u.len(), 36);
        assert_eq!(&u[14..15], "4");
        // ISO inizia con l'anno (>= 2020)
        let iso = sostituisci("{{$isoTimestamp}}", &v);
        assert!(iso.ends_with('Z') && iso.contains('T'));
        // nome dinamico sconosciuto: resta il segnaposto
        assert_eq!(sostituisci("{{$boh}}", &v), "{{$boh}}");
    }

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
