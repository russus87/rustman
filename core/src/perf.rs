//! Test di performance (carico): esegue molte richieste con un certo grado di
//! concorrenza e calcola le statistiche di latenza.

use crate::http;
use crate::model::{OpzioniPerf, Richiesta, RisultatoPerf};
use futures::stream::{self, StreamExt};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

/// Esegue `n` richieste con al massimo `concorrenza` in volo contemporaneamente.
pub async fn esegui(richiesta: &Richiesta, n: usize, concorrenza: usize) -> RisultatoPerf {
    let n = n.max(1);
    let concorrenza = concorrenza.clamp(1, 256);

    let inizio = Instant::now();

    // Ogni task misura la propria latenza e se la risposta è "ok" (status < 400).
    // `buffer_unordered` tiene in volo al più `concorrenza` richieste alla volta.
    let esiti: Vec<(u128, bool)> = stream::iter(0..n)
        .map(|_| async {
            let t = Instant::now();
            match http::invia(richiesta).await {
                Ok(r) => (t.elapsed().as_millis(), r.status < 400),
                Err(_) => (t.elapsed().as_millis(), false),
            }
        })
        .buffer_unordered(concorrenza)
        .collect()
        .await;

    let durata_totale_ms = inizio.elapsed().as_millis();
    let latenze: Vec<u128> = esiti.iter().map(|(l, _)| *l).collect();
    let ok = esiti.iter().filter(|(_, b)| *b).count();

    statistiche(latenze, ok, durata_totale_ms)
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
    let intervallo = if opz.rps > 0 {
        Some(Duration::from_secs_f64(1.0 / opz.rps as f64))
    } else {
        None
    };

    let inizio = Instant::now();
    let mut prossimo = Instant::now();
    let mut handles = Vec::new();

    while inizio.elapsed() < fine {
        // Pacing verso il RPS target.
        if let Some(iv) = intervallo {
            let ora = Instant::now();
            if ora < prossimo {
                tokio::time::sleep(prossimo - ora).await;
            }
            prossimo += iv;
        }
        let permesso = sem.clone().acquire_owned().await.unwrap();
        let r = richiesta.clone();
        let offset = inizio.elapsed();
        handles.push(tokio::spawn(async move {
            let t = Instant::now();
            let esito = http::invia(&r).await;
            let lat = t.elapsed().as_millis();
            let ok = matches!(esito, Ok(rr) if rr.status < 400);
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
    statistiche(latenze, ok, durata_ms)
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
