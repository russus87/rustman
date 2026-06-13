//! Test di performance (carico): esegue molte richieste con un certo grado di
//! concorrenza e calcola le statistiche di latenza.

use crate::http;
use crate::model::{Richiesta, RisultatoPerf};
use futures::stream::{self, StreamExt};
use std::time::Instant;

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
