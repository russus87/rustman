//! Test di performance (carico): esegue molte richieste con un certo grado di
//! concorrenza e calcola le statistiche di latenza.

use crate::fasi::{self, Accumulatore};
use crate::http;
use crate::model::{OpzioniPerf, ProgressoPerf, Richiesta, RisultatoPerf};
use futures::stream::{self, StreamExt};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

// ---------------------------------------------------------------------------
// Avanzamento del test in corso
//
// È process-globale perché di test di carico ne gira uno alla volta, e va
// letto da fuori (comando Tauri / rotta HTTP) mentre `esegui` è ancora dentro
// il suo await: passare un canale fin qui avrebbe voluto dire cambiare la
// firma di tutte le funzioni chiamanti per un dato puramente informativo.
// ---------------------------------------------------------------------------

struct StatoPerf {
    in_corso: bool,
    inizio: Option<Instant>,
    previste: usize,
    totale_ms: u128,
    completate: usize,
    ok: usize,
    somma_lat: u128,
    ultima_lat: u128,
}

impl StatoPerf {
    const fn vuoto() -> Self {
        Self {
            in_corso: false,
            inizio: None,
            previste: 0,
            totale_ms: 0,
            completate: 0,
            ok: 0,
            somma_lat: 0,
            ultima_lat: 0,
        }
    }
}

static STATO: Mutex<StatoPerf> = Mutex::new(StatoPerf::vuoto());

/// Azzera i contatori e segna il test come partito.
fn inizia(previste: usize, totale_ms: u128) {
    if let Ok(mut s) = STATO.lock() {
        *s = StatoPerf {
            in_corso: true,
            inizio: Some(Instant::now()),
            previste,
            totale_ms,
            ..StatoPerf::vuoto()
        };
    }
}

/// Registra una richiesta completata.
fn registra(lat: u128, ok: bool) {
    if let Ok(mut s) = STATO.lock() {
        s.completate += 1;
        s.somma_lat += lat;
        s.ultima_lat = lat;
        if ok {
            s.ok += 1;
        }
    }
}

fn termina() {
    if let Ok(mut s) = STATO.lock() {
        s.in_corso = false;
    }
}

/// Fotografia dell'avanzamento del test in corso (o dell'ultimo concluso).
pub fn progresso() -> ProgressoPerf {
    let Ok(s) = STATO.lock() else {
        return ProgressoPerf {
            in_corso: false,
            completate: 0,
            previste: 0,
            ok: 0,
            errori: 0,
            trascorso_ms: 0,
            totale_ms: 0,
            req_al_secondo: 0.0,
            latenza_media: 0.0,
            latenza_ultima: 0,
        };
    };
    let trascorso_ms = s.inizio.map(|i| i.elapsed().as_millis()).unwrap_or(0);
    let secondi = trascorso_ms as f64 / 1000.0;
    ProgressoPerf {
        in_corso: s.in_corso,
        completate: s.completate,
        previste: s.previste,
        ok: s.ok,
        errori: s.completate - s.ok,
        trascorso_ms,
        totale_ms: s.totale_ms,
        req_al_secondo: if secondi > 0.0 { s.completate as f64 / secondi } else { 0.0 },
        latenza_media: if s.completate > 0 { s.somma_lat as f64 / s.completate as f64 } else { 0.0 },
        latenza_ultima: s.ultima_lat,
    }
}

/// Esegue `n` richieste con al massimo `concorrenza` in volo contemporaneamente.
pub async fn esegui(richiesta: &Richiesta, n: usize, concorrenza: usize) -> RisultatoPerf {
    let n = n.max(1);
    let concorrenza = concorrenza.clamp(1, 256);

    inizia(n, 0);
    let inizio = Instant::now();

    // Le fasi che il servizio dichiara nel corpo delle risposte vengono
    // sommate qui mano a mano: tenerle tutte per poi aggregarle alla fine
    // costerebbe, su decine di migliaia di richieste, più del test stesso.
    let fasi_viste = Arc::new(Mutex::new(Accumulatore::nuovo()));

    // Ogni task misura la propria latenza e se la risposta è "ok" (status < 400).
    // `buffer_unordered` tiene in volo al più `concorrenza` richieste alla volta.
    let esiti: Vec<(u128, bool)> = stream::iter(0..n)
        .map(|_| {
            let acc = fasi_viste.clone();
            async move {
                let t = Instant::now();
                let esito = match http::invia(richiesta).await {
                    Ok(r) => {
                        // La latenza si ferma qui: l'analisi delle fasi viene
                        // dopo, altrimenti finirebbe dentro la misura.
                        let lat = t.elapsed().as_millis();
                        raccogli_fasi(&acc, &r.body);
                        (lat, r.status < 400)
                    }
                    Err(_) => (t.elapsed().as_millis(), false),
                };
                registra(esito.0, esito.1);
                esito
            }
        })
        .buffer_unordered(concorrenza)
        .collect()
        .await;

    let durata_totale_ms = inizio.elapsed().as_millis();
    termina();
    let latenze: Vec<u128> = esiti.iter().map(|(l, _)| *l).collect();
    let ok = esiti.iter().filter(|(_, b)| *b).count();

    let mut ris = statistiche(latenze, ok, durata_totale_ms);
    ris.fasi = fasi_viste.lock().map(|a| a.risultato()).unwrap_or_default();
    ris
}

/// Estrae le fasi dal corpo di una risposta e le somma all'accumulatore.
/// L'analisi avviene dopo che la latenza è stata presa, così non entra nella
/// misura; sui corpi grandi `fasi::estrai` si ferma da sola.
fn raccogli_fasi(acc: &Mutex<Accumulatore>, body: &str) {
    let fasi = fasi::estrai(body);
    if fasi.is_empty() {
        return;
    }
    if let Ok(mut a) = acc.lock() {
        a.aggiungi(&fasi);
    }
}

/// Esegue un test di carico secondo le opzioni: modo "count" (n richieste) o
/// "durata" (per `durata_s` secondi), con RPS target e warmup opzionali.
pub async fn esegui_cfg(richiesta: &Richiesta, opz: &OpzioniPerf) -> RisultatoPerf {
    let concorrenza = opz.concorrenza.clamp(1, 256);
    if opz.durata_s == 0 {
        // Modo "count": comportamento classico (nessun warmup).
        return esegui(richiesta, opz.n.max(1), concorrenza).await;
    }

    // Modo "durata": lancia richieste fino allo scadere di warmup + durata,
    // limitando i task in volo con un semaforo e (se richiesto) regolando il RPS.
    let sem = Arc::new(Semaphore::new(concorrenza));
    let warmup = Duration::from_secs(opz.warmup_s);
    let fine = Duration::from_secs(opz.warmup_s + opz.durata_s);
    // Intervalli di base e di picco (profilo "spike").
    let int_base = (opz.rps > 0).then(|| 1.0 / opz.rps as f64);
    let int_spike = (opz.spike_rps > 0)
        .then(|| 1.0 / opz.spike_rps as f64)
        .or(int_base);
    let spike = opz.profilo == "spike";

    // Nel modo "durata" le richieste non si conoscono in anticipo: l'avanzamento
    // si misura sul tempo trascorso.
    inizia(0, fine.as_millis());
    let fasi_viste = Arc::new(Mutex::new(Accumulatore::nuovo()));
    let inizio = Instant::now();
    let mut prossimo = Instant::now();
    let mut handles = Vec::new();

    while inizio.elapsed() < fine {
        // Pacing verso il RPS target; nel profilo "spike" la fascia centrale
        // (40%–60% della durata) usa il RPS di picco.
        let frazione = inizio.elapsed().as_secs_f64() / fine.as_secs_f64().max(0.001);
        let in_picco = spike && (0.4..0.6).contains(&frazione);
        let intervallo = if in_picco { int_spike } else { int_base };
        if let Some(iv) = intervallo {
            let ora = Instant::now();
            if ora < prossimo {
                tokio::time::sleep(prossimo - ora).await;
            }
            prossimo = ora.max(prossimo) + Duration::from_secs_f64(iv);
        }
        let permesso = sem.clone().acquire_owned().await.unwrap();
        let r = richiesta.clone();
        let offset = inizio.elapsed();
        let acc = fasi_viste.clone();
        handles.push(tokio::spawn(async move {
            let t = Instant::now();
            let esito = http::invia(&r).await;
            let lat = t.elapsed().as_millis();
            let ok = match &esito {
                Ok(rr) => {
                    // Le risposte del warmup non contano nelle statistiche e
                    // non contano nemmeno nelle fasi.
                    if offset >= warmup {
                        raccogli_fasi(&acc, &rr.body);
                    }
                    rr.status < 400
                }
                Err(_) => false,
            };
            registra(lat, ok);
            drop(permesso);
            // Le richieste iniziate durante il warmup non contano.
            if offset >= warmup {
                Some((lat, ok))
            } else {
                None
            }
        }));
    }

    // Attende i task ancora in volo e raccoglie le misure utili.
    let mut latenze = Vec::new();
    let mut ok = 0;
    for h in handles {
        if let Ok(Some((lat, o))) = h.await {
            latenze.push(lat);
            if o {
                ok += 1;
            }
        }
    }
    // La finestra di misura è la durata utile (esclusa il warmup).
    let durata_ms = Duration::from_secs(opz.durata_s).as_millis();
    termina();
    let mut ris = statistiche(latenze, ok, durata_ms);
    ris.fasi = fasi_viste.lock().map(|a| a.risultato()).unwrap_or_default();
    ris
}

/// Aggrega le latenze in statistiche (separata per poterla testare senza rete).
fn statistiche(latenze: Vec<u128>, ok: usize, durata_totale_ms: u128) -> RisultatoPerf {
    let totali = latenze.len();
    let errori = totali - ok;

    let mut ordinate = latenze.clone();
    ordinate.sort_unstable();

    let latenza_min = ordinate.first().copied().unwrap_or(0);
    let latenza_max = ordinate.last().copied().unwrap_or(0);
    let somma: u128 = ordinate.iter().sum();
    let latenza_media = if totali > 0 {
        somma as f64 / totali as f64
    } else {
        0.0
    };

    let secondi = durata_totale_ms as f64 / 1000.0;
    let req_al_secondo = if secondi > 0.0 {
        totali as f64 / secondi
    } else {
        0.0
    };

    RisultatoPerf {
        totali,
        ok,
        errori,
        durata_totale_ms,
        req_al_secondo,
        latenza_min,
        latenza_max,
        latenza_media,
        p50: percentile(&ordinate, 50.0),
        p90: percentile(&ordinate, 90.0),
        p95: percentile(&ordinate, 95.0),
        p99: percentile(&ordinate, 99.0),
        latenze,
        fasi: Vec::new(),
    }
}

/// Calcola il percentile `p` (0..100) su un vettore GIÀ ordinato.
fn percentile(ordinate: &[u128], p: f64) -> u128 {
    if ordinate.is_empty() {
        return 0;
    }
    let idx = ((p / 100.0) * (ordinate.len() as f64 - 1.0)).round() as usize;
    ordinate[idx.min(ordinate.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentili_e_statistiche() {
        // latenze 1..=10, tutte ok, durata 1000ms
        let latenze: Vec<u128> = (1..=10).collect();
        let s = statistiche(latenze, 10, 1000);
        assert_eq!(s.totali, 10);
        assert_eq!(s.errori, 0);
        assert_eq!(s.latenza_min, 1);
        assert_eq!(s.latenza_max, 10);
        assert_eq!(s.latenza_media, 5.5);
        assert_eq!(s.req_al_secondo, 10.0);
        assert_eq!(s.p50, 6); // indice round(0.5*9)=5 -> valore 6
        assert_eq!(s.p99, 10);
    }
}
