//! Report PDF di un test di carico, scritto a mano.
//!
//! Perché non `window.print()` della webview: nell'app desktop il dialogo di
//! stampa dipende dal backend GTK e non è garantito, mentre qui il PDF è un
//! file vero che il frontend può salvare identico su desktop e su web.
//! Perché non una libreria: serve un sottoinsieme minimo (testo Helvetica,
//! linee, rettangoli), non vale una dipendenza in più.
//!
//! Il PDF prodotto è 1.4, non compresso, una pagina A4 verticale.

use crate::model::RisultatoPerf;

const LARG: f64 = 595.0; // A4 in punti (72 dpi)
const ALT: f64 = 842.0;
const MARGINE: f64 = 48.0;

/// Colori usati nel report (r, g, b in 0..1).
const VIOLA: (f64, f64, f64) = (0.486, 0.361, 1.0);
const GRIGIO: (f64, f64, f64) = (0.45, 0.45, 0.50);
const CHIARO: (f64, f64, f64) = (0.87, 0.87, 0.90);
const NERO: (f64, f64, f64) = (0.10, 0.11, 0.15);
const ROSSO: (f64, f64, f64) = (0.90, 0.25, 0.22);
const VERDE: (f64, f64, f64) = (0.15, 0.60, 0.30);

/// Accumula il flusso di contenuto della pagina.
struct Tela {
    c: String,
}

impl Tela {
    fn nuova() -> Self {
        Tela { c: String::new() }
    }

    fn colore_testo(&mut self, (r, g, b): (f64, f64, f64)) {
        self.c.push_str(&format!("{r:.3} {g:.3} {b:.3} rg\n"));
    }
    fn colore_linea(&mut self, (r, g, b): (f64, f64, f64)) {
        self.c.push_str(&format!("{r:.3} {g:.3} {b:.3} RG\n"));
    }

    /// Testo con l'origine in basso a sinistra (sistema di coordinate PDF).
    fn testo(&mut self, x: f64, y: f64, dim: f64, grassetto: bool, t: &str) {
        let font = if grassetto { "F2" } else { "F1" };
        self.c.push_str(&format!(
            "BT /{font} {dim} Tf {x:.2} {y:.2} Td ({}) Tj ET\n",
            escapa(t)
        ));
    }

    fn linea(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, spessore: f64) {
        self.c
            .push_str(&format!("{spessore} w {x1:.2} {y1:.2} m {x2:.2} {y2:.2} l S\n"));
    }

    fn rett_pieno(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.c.push_str(&format!("{x:.2} {y:.2} {w:.2} {h:.2} re f\n"));
    }

    fn rett_bordo(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.c
            .push_str(&format!("0.7 w {x:.2} {y:.2} {w:.2} {h:.2} re S\n"));
    }

    /// Spezzata: il primo punto è un `m`, i successivi `l`.
    fn spezzata(&mut self, punti: &[(f64, f64)], spessore: f64) {
        let Some(((px, py), resto)) = punti.split_first() else { return };
        self.c.push_str(&format!("{spessore} w {px:.2} {py:.2} m\n"));
        for (x, y) in resto {
            self.c.push_str(&format!("{x:.2} {y:.2} l\n"));
        }
        self.c.push_str("S\n");
    }
}

/// Punteggiatura tipografica che in WinAnsiEncoding vive fra 0x80 e 0x9F,
/// dove Latin-1 ha invece caratteri di controllo: senza questa mappa la
/// lineetta lunga e le virgolette curve diventerebbero '?'.
fn cp1252(c: char) -> Option<char> {
    let b: u8 = match c {
        '\u{20AC}' => 0x80, // €
        '\u{201A}' => 0x82,
        '\u{201E}' => 0x84,
        '\u{2026}' => 0x85, // …
        '\u{2018}' => 0x91, // '
        '\u{2019}' => 0x92, // '
        '\u{201C}' => 0x93, // "
        '\u{201D}' => 0x94, // "
        '\u{2022}' => 0x95, // •
        '\u{2013}' => 0x96, // –
        '\u{2014}' => 0x97, // —
        _ => return None,
    };
    Some(char::from(b))
}

/// Testo dentro una stringa PDF letterale: le parentesi e il backslash vanno
/// protetti; oltre Latin-1 e la punteggiatura di `cp1252` non c'è nulla da
/// mostrare col font Helvetica standard, quindi diventa '?'.
fn escapa(t: &str) -> String {
    let mut out = String::with_capacity(t.len() + 8);
    for ch in t.chars() {
        let ch = cp1252(ch).unwrap_or(ch);
        match ch {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 32 => out.push(' '),
            c if (c as u32) < 256 => out.push(c),
            _ => out.push('?'),
        }
    }
    out
}

/// I byte della stringa in Latin-1 (WinAnsiEncoding): un carattere = un byte.
fn in_latin1(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| if (c as u32) < 256 { c as u8 } else { b'?' })
        .collect()
}

fn ms(v: f64) -> String {
    format!("{v:.0} ms")
}

/// Disegna un riquadro KPI con etichetta e valore.
fn kpi(t: &mut Tela, x: f64, y: f64, w: f64, h: f64, etichetta: &str, valore: &str) {
    t.colore_linea(CHIARO);
    t.rett_bordo(x, y, w, h);
    t.colore_testo(GRIGIO);
    t.testo(x + 10.0, y + h - 17.0, 8.5, false, etichetta);
    t.colore_testo(NERO);
    t.testo(x + 10.0, y + 12.0, 16.0, true, valore);
}

/// Grafico delle latenze in ordine di completamento.
///
/// L'asse verticale va da min a max, non da zero: su un endpoint che risponde
/// sempre attorno ai 12 s una scala 0-max schiaccerebbe tutto in una riga
/// piatta in cima, nascondendo proprio le variazioni che interessano. Le due
/// etichette dicono gli estremi, quindi la grandezza assoluta resta leggibile.
///
/// Con molti punti le colonne si riducono a 240 prendendo il massimo di ogni
/// fascia: la forma resta, il file no.
fn grafico_latenze(t: &mut Tela, x: f64, y: f64, w: f64, h: f64, lat: &[u128]) {
    t.colore_linea(CHIARO);
    t.linea(x, y, x, y + h, 0.7);
    t.linea(x, y, x + w, y, 0.7);
    if lat.is_empty() {
        return;
    }
    let min = *lat.iter().min().unwrap_or(&0) as f64;
    let max = *lat.iter().max().unwrap_or(&1) as f64;
    let span = (max - min).max(1.0);
    let colonne = 240usize.min(lat.len());
    let punti: Vec<(f64, f64)> = (0..colonne)
        .map(|i| {
            let da = i * lat.len() / colonne;
            let a = ((i + 1) * lat.len() / colonne).max(da + 1);
            let v = lat[da..a.min(lat.len())].iter().copied().max().unwrap_or(0) as f64;
            let px = x + if colonne == 1 { 0.0 } else { i as f64 / (colonne - 1) as f64 * w };
            (px, y + ((v - min) / span) * h)
        })
        .collect();
    t.colore_linea(VIOLA);
    t.spezzata(&punti, 1.2);
    t.colore_testo(GRIGIO);
    t.testo(x - 34.0, y + h - 4.0, 7.5, false, &format!("{max:.0}"));
    t.testo(x - 34.0, y - 1.0, 7.5, false, &format!("{min:.0}"));
}

/// Istogramma della distribuzione delle latenze.
fn grafico_istogramma(t: &mut Tela, x: f64, y: f64, w: f64, h: f64, lat: &[u128], nb: usize) {
    t.colore_linea(CHIARO);
    t.linea(x, y, x, y + h, 0.7);
    t.linea(x, y, x + w, y, 0.7);
    if lat.is_empty() {
        return;
    }
    let min = *lat.iter().min().unwrap_or(&0) as f64;
    let max = *lat.iter().max().unwrap_or(&1) as f64;
    let range = (max - min).max(1.0);
    let mut fasce = vec![0usize; nb];
    for v in lat {
        let mut i = (((*v as f64 - min) / range) * nb as f64) as usize;
        if i >= nb {
            i = nb - 1;
        }
        fasce[i] += 1;
    }
    let picco = *fasce.iter().max().unwrap_or(&1).max(&1) as f64;
    let bw = w / nb as f64;
    t.colore_testo(VIOLA);
    for (i, c) in fasce.iter().enumerate() {
        let bh = (*c as f64 / picco) * h;
        if bh > 0.0 {
            t.rett_pieno(x + i as f64 * bw + 1.0, y, bw - 2.0, bh);
        }
    }
    t.colore_testo(GRIGIO);
    t.testo(x, y - 11.0, 7.5, false, &ms(min));
    t.testo(x + w - 34.0, y - 11.0, 7.5, false, &ms(max));
}

/// Costruisce il PDF del report. `titolo` e `sottotitolo` descrivono la
/// richiesta testata; `parametri` sono le righe "chiave: valore" della config.
pub fn genera(
    r: &RisultatoPerf,
    titolo: &str,
    sottotitolo: &str,
    parametri: &[(String, String)],
) -> Vec<u8> {
    let mut t = Tela::nuova();
    let mut y = ALT - MARGINE;

    // Intestazione
    t.colore_testo(VIOLA);
    t.testo(MARGINE, y - 16.0, 20.0, true, "Rustman — report di carico");
    y -= 34.0;
    t.colore_testo(NERO);
    t.testo(MARGINE, y - 12.0, 12.0, true, titolo);
    y -= 18.0;
    t.colore_testo(GRIGIO);
    t.testo(MARGINE, y - 10.0, 9.0, false, sottotitolo);
    y -= 20.0;
    t.colore_linea(CHIARO);
    t.linea(MARGINE, y, LARG - MARGINE, y, 0.7);
    y -= 22.0;

    // Riquadri KPI (due file da quattro)
    let utile = LARG - 2.0 * MARGINE;
    let bw = (utile - 3.0 * 10.0) / 4.0;
    let bh = 46.0;
    let riga1 = [
        ("RICHIESTE", r.totali.to_string()),
        ("OK", r.ok.to_string()),
        ("ERRORI", r.errori.to_string()),
        ("REQ/S", format!("{:.1}", r.req_al_secondo)),
    ];
    for (i, (e, v)) in riga1.iter().enumerate() {
        kpi(&mut t, MARGINE + i as f64 * (bw + 10.0), y - bh, bw, bh, e, v);
    }
    y -= bh + 12.0;
    let riga2 = [
        ("LATENZA MIN", ms(r.latenza_min as f64)),
        ("LATENZA MEDIA", ms(r.latenza_media)),
        ("P95", ms(r.p95 as f64)),
        ("LATENZA MAX", ms(r.latenza_max as f64)),
    ];
    for (i, (e, v)) in riga2.iter().enumerate() {
        kpi(&mut t, MARGINE + i as f64 * (bw + 10.0), y - bh, bw, bh, e, v);
    }
    y -= bh + 24.0;

    // Percentili
    t.colore_testo(NERO);
    t.testo(MARGINE, y, 10.0, true, "Percentili");
    y -= 14.0;
    t.colore_testo(GRIGIO);
    t.testo(
        MARGINE,
        y,
        9.5,
        false,
        &format!(
            "P50 {}   ·   P90 {}   ·   P95 {}   ·   P99 {}   ·   durata totale {:.2} s",
            ms(r.p50 as f64),
            ms(r.p90 as f64),
            ms(r.p95 as f64),
            ms(r.p99 as f64),
            r.durata_totale_ms as f64 / 1000.0
        ),
    );
    y -= 26.0;

    // Grafico latenze
    t.colore_testo(NERO);
    t.testo(MARGINE, y, 10.0, true, "Latenza per richiesta");
    y -= 12.0;
    let gh = 120.0;
    grafico_latenze(&mut t, MARGINE + 34.0, y - gh, utile - 34.0, gh, &r.latenze);
    y -= gh + 14.0;
    t.colore_testo(GRIGIO);
    t.testo(MARGINE + 34.0, y, 7.5, false, "in ordine di completamento (picco per fascia)");
    y -= 28.0;

    // Istogramma
    t.colore_testo(NERO);
    t.testo(MARGINE, y, 10.0, true, "Distribuzione delle latenze");
    y -= 12.0;
    let ih = 110.0;
    grafico_istogramma(&mut t, MARGINE + 34.0, y - ih, utile - 34.0, ih, &r.latenze, 24);
    y -= ih + 36.0;

    // Parametri del test
    t.colore_testo(NERO);
    t.testo(MARGINE, y, 10.0, true, "Parametri");
    y -= 15.0;
    t.colore_testo(GRIGIO);
    for (k, v) in parametri {
        if y < MARGINE + 14.0 {
            break;
        }
        t.testo(MARGINE, y, 9.0, false, &format!("{k}: {v}"));
        y -= 12.5;
    }

    // Piede: la percentuale di errori in rosso quando ce ne sono.
    if r.errori > 0 {
        t.colore_testo(ROSSO);
        let perc = r.errori as f64 / (r.totali.max(1)) as f64 * 100.0;
        t.testo(MARGINE, MARGINE - 12.0, 9.0, true, &format!("{perc:.1}% di richieste in errore"));
    }

    assembla(std::slice::from_ref(&t.c))
}

/// Mette insieme gli oggetti PDF e la tabella xref.
/// Una pagina per elemento di `pagine`; i due font sono condivisi.
///
/// Numerazione degli oggetti: 1 catalogo, 2 albero pagine, 3 e 4 i font, poi
/// per ogni pagina la coppia /Page + /Contents.
fn assembla(pagine: &[String]) -> Vec<u8> {
    let n = pagine.len().max(1);
    let primo_page = 5; // gli oggetti 1..4 sono catalogo, pagine e i due font
    let kids: Vec<String> = (0..n)
        .map(|i| format!("{} 0 R", primo_page + i * 2))
        .collect();

    let mut oggetti: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")).into_bytes(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>"
            .to_vec(),
    ];
    let vuota = String::new();
    for i in 0..n {
        let contenuto = pagine.get(i).unwrap_or(&vuota);
        let flusso = in_latin1(contenuto);
        oggetti.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {LARG} {ALT}] \
                 /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {} 0 R >>",
                primo_page + i * 2 + 1
            )
            .into_bytes(),
        );
        let mut v = format!("<< /Length {} >>\nstream\n", flusso.len()).into_bytes();
        v.extend_from_slice(&flusso);
        v.extend_from_slice(b"\nendstream");
        oggetti.push(v);
    }

    let mut out: Vec<u8> = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offset = Vec::with_capacity(oggetti.len());
    for (i, o) in oggetti.iter().enumerate() {
        offset.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(o);
        out.extend_from_slice(b"\nendobj\n");
    }

    let inizio_xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", oggetti.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offset {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{inizio_xref}\n%%EOF\n",
            oggetti.len() + 1
        )
        .as_bytes(),
    );
    out
}

// ===================== Report di un flusso con loop =========================

/// Un nodo del flusso eseguito in loop, così come arriva dal frontend.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SezioneRun {
    pub nome: String,
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub giri: usize,
    #[serde(default)]
    pub ok_giri: usize,
    #[serde(default)]
    pub falliti: usize,
    #[serde(default)]
    pub concorrenza: usize,
    /// "volte" oppure "foreach <path>".
    #[serde(default)]
    pub sorgente: String,
    /// Perché il ciclo è finito (completato / condizione di uscita / …).
    #[serde(default)]
    pub uscita: String,
    /// Latenza di ogni giro, in ordine di giro.
    #[serde(default)]
    pub tempi: Vec<u128>,
    #[serde(default)]
    pub test_ok: usize,
    #[serde(default)]
    pub test_tot: usize,
    /// Righe di dettaglio sui giri falliti.
    #[serde(default)]
    pub problemi: Vec<String>,
}

/// Statistiche di latenza di una sezione. Stessa convenzione dei percentili di
/// `perf::percentile`, così i numeri combaciano col pannello Performance.
struct Stat {
    min: u128,
    max: u128,
    media: f64,
    p50: u128,
    p90: u128,
    p95: u128,
    p99: u128,
    totale: u128,
}

fn perc(ordinati: &[u128], p: f64) -> u128 {
    if ordinati.is_empty() {
        return 0;
    }
    let i = ((p / 100.0) * (ordinati.len() as f64 - 1.0)).round() as usize;
    ordinati[i.min(ordinati.len() - 1)]
}

fn stat(tempi: &[u128]) -> Stat {
    let mut o = tempi.to_vec();
    o.sort_unstable();
    let totale: u128 = o.iter().sum();
    Stat {
        min: o.first().copied().unwrap_or(0),
        max: o.last().copied().unwrap_or(0),
        media: if o.is_empty() { 0.0 } else { totale as f64 / o.len() as f64 },
        p50: perc(&o, 50.0),
        p90: perc(&o, 90.0),
        p95: perc(&o, 95.0),
        p99: perc(&o, 99.0),
        totale,
    }
}

/// Taglia una stringa a `max` caratteri, con i puntini se serve: il font è a
/// larghezza variabile e senza metriche non si può misurare il testo, quindi
/// si limita il numero di caratteri.
fn taglia(t: &str, max: usize) -> String {
    if t.chars().count() <= max {
        return t.to_string();
    }
    let mut out: String = t.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Report PDF di un flusso eseguito: una fascia di confronto fra i nodi con
/// loop, poi una scheda per nodo con statistiche, andamento e distribuzione.
pub fn genera_run(titolo: &str, sottotitolo: &str, sezioni: &[SezioneRun]) -> Vec<u8> {
    let utile = LARG - 2.0 * MARGINE;
    let mut pagine: Vec<String> = Vec::new();
    let mut t = Tela::nuova();
    let mut y = ALT - MARGINE;

    // Intestazione della prima pagina.
    t.colore_testo(VIOLA);
    t.testo(MARGINE, y - 16.0, 20.0, true, "Rustman — report del flusso");
    y -= 34.0;
    t.colore_testo(NERO);
    t.testo(MARGINE, y - 12.0, 12.0, true, titolo);
    y -= 18.0;
    t.colore_testo(GRIGIO);
    t.testo(MARGINE, y - 10.0, 9.0, false, sottotitolo);
    y -= 20.0;
    t.colore_linea(CHIARO);
    t.linea(MARGINE, y, LARG - MARGINE, y, 0.7);
    y -= 24.0;

    // --- Fascia di confronto: solo se c'è più di un nodo con dei tempi ---
    let confrontabili: Vec<&SezioneRun> = sezioni.iter().filter(|s| !s.tempi.is_empty()).collect();
    if confrontabili.len() > 1 {
        t.colore_testo(NERO);
        t.testo(MARGINE, y, 11.0, true, "Confronto");
        y -= 16.0;
        let col = [0.0, 200.0, 240.0, 285.0, 335.0, 385.0, 440.0];
        t.colore_testo(GRIGIO);
        for (i, e) in ["nodo", "giri", "ok", "P50", "P95", "media", "vs 1°"].iter().enumerate() {
            t.testo(MARGINE + col[i], y, 8.0, true, e);
        }
        y -= 4.0;
        t.colore_linea(CHIARO);
        t.linea(MARGINE, y, LARG - MARGINE, y, 0.5);
        y -= 12.0;

        let base = stat(&confrontabili[0].tempi).p50 as f64;
        let mut stats = Vec::new();
        for s in &confrontabili {
            let st = stat(&s.tempi);
            t.colore_testo(NERO);
            t.testo(MARGINE + col[0], y, 9.0, false, &taglia(&s.nome, 34));
            t.colore_testo(GRIGIO);
            t.testo(MARGINE + col[1], y, 9.0, false, &s.giri.to_string());
            t.testo(MARGINE + col[2], y, 9.0, false, &s.ok_giri.to_string());
            t.testo(MARGINE + col[3], y, 9.0, false, &ms(st.p50 as f64));
            t.testo(MARGINE + col[4], y, 9.0, false, &ms(st.p95 as f64));
            t.testo(MARGINE + col[5], y, 9.0, false, &ms(st.media));
            if stats.is_empty() {
                t.colore_testo(GRIGIO);
                t.testo(MARGINE + col[6], y, 9.0, false, "riferimento");
            } else if base > 0.0 {
                let d = (st.p50 as f64 - base) / base * 100.0;
                // Più lento del riferimento = rosso, altrimenti verde.
                t.colore_testo(if d > 0.5 { ROSSO } else { VERDE });
                t.testo(MARGINE + col[6], y, 9.0, false, &format!("{d:+.1}%"));
            }
            stats.push(st);
            y -= 13.0;
        }
        y -= 8.0;

        // Barre proporzionali alla mediana: colpo d'occhio su chi è più lento.
        let peggiore = stats.iter().map(|s| s.p50).max().unwrap_or(1).max(1) as f64;
        for (s, st) in confrontabili.iter().zip(stats.iter()) {
            let w = (st.p50 as f64 / peggiore) * (utile - 150.0);
            t.colore_testo(VIOLA);
            t.rett_pieno(MARGINE + 150.0, y - 8.0, w.max(1.0), 9.0);
            t.colore_testo(GRIGIO);
            t.testo(MARGINE, y - 7.0, 8.5, false, &taglia(&s.nome, 28));
            y -= 14.0;
        }
        y -= 16.0;
    }

    // --- Una scheda per nodo ---
    // Altezza reale di una scheda, tenuta allineata al codice qui sotto:
    // 15 + 13 + 14 + (72+11) + 18 + (54+24) + 14 + 14.
    const ALTEZZA_SCHEDA: f64 = 249.0;
    for s in sezioni {
        if y - ALTEZZA_SCHEDA < MARGINE {
            pagine.push(std::mem::take(&mut t.c));
            t = Tela::nuova();
            y = ALT - MARGINE;
        }
        let st = stat(&s.tempi);
        t.colore_testo(if s.ok { VERDE } else { ROSSO });
        t.testo(MARGINE, y, 8.5, true, if s.ok { "OK" } else { "FAIL" });
        t.colore_testo(NERO);
        t.testo(MARGINE + 30.0, y, 11.0, true, &taglia(&s.nome, 52));
        y -= 15.0;
        t.colore_testo(GRIGIO);
        t.testo(
            MARGINE,
            y,
            8.5,
            false,
            &format!(
                "{} giri ({}) · {} in parallelo · {} ok · {} falliti · test {}/{} · {}",
                s.giri, s.sorgente, s.concorrenza.max(1), s.ok_giri, s.falliti,
                s.test_ok, s.test_tot, s.uscita
            ),
        );
        y -= 13.0;
        if s.tempi.is_empty() {
            t.testo(MARGINE, y, 8.5, false, "nessun tempo registrato");
            y -= 26.0;
            continue;
        }
        t.testo(
            MARGINE,
            y,
            8.5,
            false,
            &format!(
                "min {} · P50 {} · P90 {} · P95 {} · P99 {} · max {} · media {} · totale {:.2} s",
                ms(st.min as f64), ms(st.p50 as f64), ms(st.p90 as f64), ms(st.p95 as f64),
                ms(st.p99 as f64), ms(st.max as f64), ms(st.media),
                st.totale as f64 / 1000.0
            ),
        );
        y -= 14.0;

        let gh = 72.0;
        grafico_latenze(&mut t, MARGINE + 34.0, y - gh, utile - 34.0, gh, &s.tempi);
        y -= gh + 11.0;
        t.colore_testo(GRIGIO);
        t.testo(MARGINE + 34.0, y, 7.5, false, "latenza per giro");
        y -= 18.0;

        let ih = 54.0;
        let fasce = 16.min(s.tempi.len().max(1));
        grafico_istogramma(&mut t, MARGINE + 34.0, y - ih, utile - 34.0, ih, &s.tempi, fasce);
        y -= ih + 24.0; // spazio per le etichette min/max dell'istogramma
        t.colore_testo(GRIGIO);
        t.testo(MARGINE + 34.0, y, 7.5, false, "distribuzione");
        y -= 14.0;

        for riga in s.problemi.iter().take(6) {
            if y < MARGINE + 14.0 {
                break;
            }
            t.colore_testo(ROSSO);
            t.testo(MARGINE, y, 8.0, false, &taglia(riga, 110));
            y -= 11.0;
        }
        y -= 14.0;
    }

    pagine.push(std::mem::take(&mut t.c));
    assembla(&pagine)
}

/// Base64 standard: serve per consegnare i byte del PDF al frontend dentro il
/// JSON della risposta, dove un `Vec<u8>` diventerebbe un array di numeri
/// (circa quattro volte più grande della stringa base64).
pub fn base64(dati: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(dati.len().div_ceil(3) * 4);
    for blocco in dati.chunks(3) {
        let b = [blocco[0], *blocco.get(1).unwrap_or(&0), *blocco.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(A[(n >> 18) as usize & 63] as char);
        out.push(A[(n >> 12) as usize & 63] as char);
        out.push(if blocco.len() > 1 { A[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if blocco.len() > 2 { A[n as usize & 63] as char } else { '=' });
    }
    out
}

#[cfg(test)]
mod test {
    use super::*;

    fn finto(n: usize) -> RisultatoPerf {
        let latenze: Vec<u128> = (0..n).map(|i| (10 + (i % 40)) as u128).collect();
        RisultatoPerf {
            totali: n,
            ok: n.saturating_sub(2),
            errori: 2.min(n),
            durata_totale_ms: 1500,
            req_al_secondo: 33.3,
            latenza_min: 10,
            latenza_max: 49,
            latenza_media: 29.5,
            p50: 30,
            p90: 45,
            p95: 47,
            p99: 49,
            latenze,
        }
    }

    #[test]
    fn produce_un_pdf_valido() {
        let pdf = genera(&finto(500), "Login", "POST https://api.esempio.it/login", &[
            ("Concorrenza".into(), "10".into()),
        ]);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
        // La xref dichiarata deve puntare davvero alla parola "xref".
        let s = String::from_utf8_lossy(&pdf);
        let pos: usize = s.rsplit("startxref").next().unwrap().trim()
            .lines().next().unwrap().trim().parse().unwrap();
        assert_eq!(&pdf[pos..pos + 4], b"xref");
    }

    #[test]
    fn regge_un_risultato_vuoto() {
        let mut r = finto(0);
        r.totali = 0;
        r.ok = 0;
        r.errori = 0;
        let pdf = genera(&r, "Vuoto", "", &[]);
        assert!(pdf.starts_with(b"%PDF-1.4"));
    }

    fn sezione(nome: &str, n: usize, base: u128) -> SezioneRun {
        SezioneRun {
            nome: nome.into(),
            ok: true,
            giri: n,
            ok_giri: n,
            falliti: 0,
            concorrenza: 10,
            sorgente: "volte".into(),
            uscita: "completato".into(),
            tempi: (0..n).map(|i| base + (i as u128 % 7) * 20).collect(),
            test_ok: n,
            test_tot: n,
            problemi: vec![],
        }
    }

    #[test]
    fn report_run_multipagina() {
        // Abbastanza sezioni da costringere il report su più pagine.
        let sezioni: Vec<SezioneRun> = (0..7)
            .map(|i| sezione(&format!("Nodo {i}"), 40, 100 + i as u128 * 30))
            .collect();
        let pdf = genera_run("Massivo", "7 nodi", &sezioni);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        let s = String::from_utf8_lossy(&pdf);
        let conteggio: usize = s
            .split("/Count ")
            .nth(1)
            .and_then(|x| x.split_whitespace().next())
            .unwrap()
            .parse()
            .unwrap();
        assert!(conteggio > 1, "atteso più di una pagina, trovate {conteggio}");
        // Ogni /Page dichiarata deve esistere davvero come oggetto.
        assert_eq!(s.matches("/Type /Page ").count(), conteggio);
        // La xref finale deve puntare alla parola "xref".
        let pos: usize = s.rsplit("startxref").next().unwrap().trim()
            .lines().next().unwrap().trim().parse().unwrap();
        assert_eq!(&pdf[pos..pos + 4], b"xref");
    }

    #[test]
    fn report_run_senza_sezioni() {
        let pdf = genera_run("Vuoto", "", &[]);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
    }

    #[test]
    fn report_run_con_un_nodo_senza_tempi() {
        let mut s = sezione("Saltato", 0, 0);
        s.ok = false;
        s.problemi = vec!["giro 2: status 500".into()];
        let pdf = genera_run("Uno", "", &[s]);
        assert!(pdf.starts_with(b"%PDF-1.4"));
    }

    #[test]
    fn base64_come_da_rfc() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0u8, 255, 128]), "AP+A");
    }

    #[test]
    fn protegge_le_parentesi_nel_testo() {
        assert_eq!(escapa("a(b)c\\d"), "a\\(b\\)c\\\\d");
        assert_eq!(escapa("caffè"), "caffè");
        assert_eq!(escapa("emoji 🎉"), "emoji ?");
        // La lineetta lunga esiste in WinAnsi: non deve diventare '?'.
        assert_eq!(escapa("a — b"), "a \u{97} b");
    }
}
