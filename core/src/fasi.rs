//! Fasi dichiarate dal servizio dentro il corpo della risposta.
//!
//! Molte API raccontano quanto è durata ogni tappa della loro elaborazione,
//! di solito in un array di log:
//!
//! ```json
//! { "log": ["Step: LoadFontStep completed in 349.9064 ms",
//!           "Step: ToPsProcessStep completed in 1822.2507 ms"] }
//! ```
//!
//! Qui quelle righe tornano a essere numeri, così la UI può dire quanto pesa
//! ogni fase sul totale. Se nel corpo non c'è niente di riconoscibile la lista
//! è vuota e nessuno mostra niente: la funzione non inventa fasi.
//!
//! Le stesse regole sono implementate in `src/lib/fasi.js`, che le applica al
//! corpo di una singola risposta (lì il JSON è già stato analizzato dal
//! pannello Response e attraversarlo di nuovo qui costerebbe un giro di IPC su
//! un corpo che può pesare megabyte). Se cambi cosa viene riconosciuto,
//! cambialo in tutti e due i posti.

use crate::model::FasePerf;
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Una tappa dichiarata dal servizio, con la sua durata in millisecondi.
#[derive(Debug, Clone, PartialEq)]
pub struct Fase {
    pub nome: String,
    pub ms: f64,
}

/// Oltre questa dimensione il corpo non viene esaminato: durante un test di
/// carico attraversare decine di MB per ogni richiesta falserebbe la misura
/// che il test sta prendendo.
pub const MAX_CORPO: usize = 256 * 1024;

/// Quante fasi distinte al massimo tiene un [`Accumulatore`]. Serve solo a
/// mettere un tetto se i nomi contengono un id e sono diversi a ogni giro.
const MAX_DISTINTE: usize = 200;

/// Parole che separano il nome della fase dalla sua durata.
const SEPARATORI: [&str; 7] = [
    "completed in",
    "completata in",
    "completato in",
    "finished in",
    "elapsed in",
    "eseguita in",
    "took",
];

/// Prefissi che introducono il nome della fase in una riga di log: tutto ciò
/// che li precede (timestamp, livello di log) non fa parte del nome.
const PREFISSI: [&str; 4] = ["step:", "fase:", "phase:", "task:"];

/// Chiavi che, in un oggetto JSON, portano il nome della fase.
const CHIAVI_NOME: [&str; 9] = [
    "step", "fase", "phase", "tappa", "task", "name", "nome", "label", "etichetta",
];

/// Estrae le fasi dal corpo di una risposta.
///
/// Se il corpo è JSON valido vengono esaminate tutte le stringhe (a qualunque
/// profondità) e gli oggetti che portano un nome e una durata; altrimenti il
/// corpo viene letto riga per riga come se fosse un log.
pub fn estrai(corpo: &str) -> Vec<Fase> {
    if corpo.is_empty() || corpo.len() > MAX_CORPO {
        return Vec::new();
    }
    if let Ok(v) = serde_json::from_str::<Value>(corpo) {
        let mut out = Vec::new();
        cammina(&v, &mut out);
        return out;
    }
    corpo.lines().filter_map(da_riga).collect()
}

/// Attraversa il JSON raccogliendo le fasi che incontra, in ordine di lettura.
fn cammina(v: &Value, out: &mut Vec<Fase>) {
    match v {
        Value::String(s) => {
            if let Some(f) = da_riga(s) {
                out.push(f);
            }
        }
        Value::Array(a) => {
            for x in a {
                cammina(x, out);
            }
        }
        Value::Object(o) => {
            // Un oggetto che è già una fase non viene riaperto: i suoi campi
            // sono il nome e la durata che abbiamo appena letto.
            if let Some(f) = da_oggetto(o) {
                out.push(f);
            } else {
                for x in o.values() {
                    cammina(x, out);
                }
            }
        }
        _ => {}
    }
}

/// Riconosce una riga tipo `Step: LoadFontStep completed in 349.9064 ms`.
fn da_riga(riga: &str) -> Option<Fase> {
    // `to_ascii_lowercase` non cambia la lunghezza in byte delle sequenze
    // UTF-8, quindi gli indici trovati qui valgono anche sulla riga originale.
    let basso = riga.to_ascii_lowercase();
    let (pos, sep) = SEPARATORI
        .iter()
        .filter_map(|s| basso.find(s).map(|p| (p, *s)))
        .min_by_key(|(p, _)| *p)?;
    let ms = durata_ms(riga[pos + sep.len()..].trim_start())?;

    // Il nome è quello che segue "Step:" (se c'è), altrimenti tutto quello che
    // precede il separatore.
    let inizio = PREFISSI
        .iter()
        .filter_map(|p| basso[..pos].rfind(p).map(|i| i + p.len()))
        .max()
        .unwrap_or(0);
    let nome = riga[inizio..pos]
        .trim()
        .trim_matches(|c: char| c == '"' || c == ',' || c == '-' || c == '|')
        .trim();
    (!nome.is_empty() && nome.chars().count() <= 120).then(|| Fase {
        nome: nome.to_string(),
        ms,
    })
}

/// Riconosce un oggetto tipo `{ "step": "LoadFont", "durationMs": 349.9 }`.
fn da_oggetto(o: &Map<String, Value>) -> Option<Fase> {
    let mut nome = None;
    let mut ms = None;
    for (k, v) in o {
        let k = normalizza(k);
        if nome.is_none() && CHIAVI_NOME.contains(&k.as_str()) {
            nome = v.as_str().map(str::to_string);
        }
        if ms.is_none() {
            if let Some(fattore) = fattore_chiave(&k) {
                ms = v.as_f64().map(|x| x * fattore);
            }
        }
    }
    let (nome, ms) = (nome?, ms?);
    let nome = nome.trim();
    (!nome.is_empty() && nome.chars().count() <= 120 && ms.is_finite() && ms >= 0.0).then(|| Fase {
        nome: nome.to_string(),
        ms,
    })
}

/// Chiave di un oggetto ridotta alla forma confrontabile: minuscola e senza
/// separatori, così `durationMs`, `duration_ms` e `DURATION-MS` coincidono.
fn normalizza(k: &str) -> String {
    k.chars()
        .filter(|c| *c != '_' && *c != '-' && *c != ' ')
        .flat_map(char::to_lowercase)
        .collect()
}

/// Quanti millisecondi vale il valore di una chiave di durata (`None` se la
/// chiave non è una durata).
fn fattore_chiave(k: &str) -> Option<f64> {
    match k {
        "ms" | "millis" | "milliseconds" | "millisecondi" | "durationms" | "duratams"
        | "elapsedms" | "tempoms" | "timems" | "tookms" | "duration" | "durata" | "elapsed"
        | "took" => Some(1.0),
        "s" | "sec" | "seconds" | "secondi" | "durations" | "duratas" | "durationseconds"
        | "elapsedseconds" => Some(1000.0),
        _ => None,
    }
}

/// Legge una durata con la sua unità: `349.9064 ms` → 349.9064, `1,5 s` → 1500.
/// Senza unità restituisce `None`: un numero da solo non è una durata, e
/// prenderlo per tale riempirebbe l'elenco di fasi inventate.
fn durata_ms(t: &str) -> Option<f64> {
    let grezzo: String = t
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
        .collect();
    let numero = grezzo.trim_end_matches(['.', ',']);
    if numero.is_empty() {
        return None;
    }
    // "1.234,56" e "1,234.56": il separatore che compare per ultimo è quello
    // dei decimali, l'altro raggruppa le migliaia e va tolto.
    let pulito = match (numero.rfind('.'), numero.rfind(',')) {
        (Some(p), Some(v)) if p > v => numero.replace(',', ""),
        (Some(p), Some(v)) if v > p => numero.replace('.', "").replace(',', "."),
        _ => numero.replace(',', "."),
    };
    let valore: f64 = pulito.parse().ok()?;

    let resto = &t[grezzo.len()..];
    let unita: String = resto
        .trim_start()
        .chars()
        .take_while(|c| c.is_alphabetic() || *c == 'µ')
        .flat_map(char::to_lowercase)
        .collect();
    let fattore = match unita.as_str() {
        "ms" | "msec" | "millis" | "millisecondi" | "milliseconds" => 1.0,
        "s" | "sec" | "secs" | "second" | "seconds" | "secondi" => 1000.0,
        "us" | "µs" | "microsecondi" | "microseconds" => 0.001,
        "ns" => 0.000_001,
        "m" | "min" | "minuti" | "minutes" => 60_000.0,
        _ => return None,
    };
    Some(valore * fattore)
}

// ---------------------------------------------------------------------------
// Aggregazione per il test di carico
// ---------------------------------------------------------------------------

/// Somme parziali di una fase durante un test di carico.
#[derive(Default)]
struct Voce {
    occorrenze: usize,
    somma: f64,
    min: f64,
    max: f64,
}

/// Accumula le fasi viste durante un test di carico tenendo solo i totali: in
/// un test da decine di migliaia di richieste conservare ogni singola misura
/// costerebbe più della misura stessa.
#[derive(Default)]
pub struct Accumulatore {
    /// Nomi in ordine di prima comparsa: è l'ordine della pipeline, e come
    /// tale è più leggibile di qualunque riordino alfabetico.
    ordine: Vec<String>,
    dati: HashMap<String, Voce>,
}

impl Accumulatore {
    pub fn nuovo() -> Self {
        Self::default()
    }

    /// Registra le fasi di una risposta.
    pub fn aggiungi(&mut self, fasi: &[Fase]) {
        for f in fasi {
            let voce = match self.dati.get_mut(&f.nome) {
                Some(v) => v,
                None => {
                    if self.ordine.len() >= MAX_DISTINTE {
                        continue;
                    }
                    self.ordine.push(f.nome.clone());
                    self.dati.entry(f.nome.clone()).or_insert(Voce {
                        occorrenze: 0,
                        somma: 0.0,
                        min: f64::MAX,
                        max: 0.0,
                    })
                }
            };
            voce.occorrenze += 1;
            voce.somma += f.ms;
            voce.min = voce.min.min(f.ms);
            voce.max = voce.max.max(f.ms);
        }
    }

    /// Fasi aggregate, con la quota di ciascuna sul tempo medio complessivo.
    pub fn risultato(&self) -> Vec<FasePerf> {
        let medie: Vec<f64> = self
            .ordine
            .iter()
            .map(|n| {
                let v = &self.dati[n];
                v.somma / v.occorrenze.max(1) as f64
            })
            .collect();
        let totale: f64 = medie.iter().sum();
        self.ordine
            .iter()
            .zip(medie)
            .map(|(nome, ms_medio)| {
                let v = &self.dati[nome];
                FasePerf {
                    nome: nome.clone(),
                    occorrenze: v.occorrenze,
                    ms_medio,
                    ms_min: if v.min == f64::MAX { 0.0 } else { v.min },
                    ms_max: v.max,
                    ms_totale: v.somma,
                    quota: if totale > 0.0 {
                        ms_medio / totale * 100.0
                    } else {
                        0.0
                    },
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legge_le_righe_di_log_nel_json() {
        let corpo = r#"{"esito":"ok","log":[
            "Step: CreateWorkFolderStep completed in 1097.1401 ms",
            "Step: LoadFontStep completed in 349.9064 ms",
            "Step: ToOutProcessStep completed in 6386.2173 ms"
        ]}"#;
        let f = estrai(corpo);
        assert_eq!(f.len(), 3);
        assert_eq!(f[0].nome, "CreateWorkFolderStep");
        assert!((f[0].ms - 1097.1401).abs() < 1e-6);
        assert_eq!(f[2].nome, "ToOutProcessStep");
    }

    #[test]
    fn niente_fasi_niente_risultato() {
        assert!(estrai(r#"{"utenti":[{"id":1,"nome":"Ada"}]}"#).is_empty());
        assert!(estrai("").is_empty());
        assert!(estrai("un testo qualsiasi, 200 ok").is_empty());
        // Un numero senza unità non è una durata.
        assert!(estrai(r#"["Step: Tal completed in 12"]"#).is_empty());
    }

    #[test]
    fn riconosce_oggetti_con_nome_e_durata() {
        let corpo = r#"{"steps":[{"name":"parse","durationMs":12.5},
                                 {"name":"render","duration_ms":37.5}]}"#;
        let f = estrai(corpo);
        assert_eq!(f.len(), 2);
        assert_eq!(f[1].nome, "render");
        assert!((f[1].ms - 37.5).abs() < 1e-9);
    }

    #[test]
    fn unita_e_prefissi_diversi() {
        // Timestamp e livello davanti al nome; secondi al posto dei ms.
        let f = da_riga("2026-09-09 10:00:01 INFO  Step: ToPs took 1,5 s").unwrap();
        assert_eq!(f.nome, "ToPs");
        assert!((f.ms - 1500.0).abs() < 1e-9);
        // Migliaia col punto e decimali con la virgola.
        let f = da_riga("Fase: Export completed in 1.822,25 ms").unwrap();
        assert!((f.ms - 1822.25).abs() < 1e-9);
        // Senza prefisso: il nome è tutto quello che precede.
        let f = da_riga("LoadFontStep completed in 349 ms").unwrap();
        assert_eq!(f.nome, "LoadFontStep");
    }

    #[test]
    fn corpo_non_json_letto_come_log() {
        let corpo = "avvio\nStep: uno completed in 10 ms\nStep: due completed in 30 ms\nfine";
        let f = estrai(corpo);
        assert_eq!(f.len(), 2);
        assert_eq!(f[1].ms, 30.0);
    }

    #[test]
    fn corpo_troppo_grande_non_viene_esaminato() {
        let riempimento = "x".repeat(MAX_CORPO);
        let corpo = format!("[\"Step: uno completed in 10 ms\",\"{riempimento}\"]");
        assert!(estrai(&corpo).is_empty());
    }

    #[test]
    fn aggrega_medie_e_quote() {
        let mut a = Accumulatore::nuovo();
        a.aggiungi(&[
            Fase { nome: "uno".into(), ms: 100.0 },
            Fase { nome: "due".into(), ms: 300.0 },
        ]);
        a.aggiungi(&[
            Fase { nome: "uno".into(), ms: 200.0 },
            Fase { nome: "due".into(), ms: 300.0 },
        ]);
        let r = a.risultato();
        assert_eq!(r.len(), 2);
        // Ordine di prima comparsa, non alfabetico né per durata.
        assert_eq!(r[0].nome, "uno");
        assert_eq!(r[0].occorrenze, 2);
        assert_eq!(r[0].ms_medio, 150.0);
        assert_eq!(r[0].ms_min, 100.0);
        assert_eq!(r[0].ms_max, 200.0);
        // 150 su 450 di media complessiva.
        assert!((r[0].quota - 33.333).abs() < 0.01);
        assert!((r[1].quota - 66.666).abs() < 0.01);
    }

    #[test]
    fn accumulatore_vuoto() {
        assert!(Accumulatore::nuovo().risultato().is_empty());
    }
}
