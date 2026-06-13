//! Diff testuale riga-per-riga (LCS), usato per confrontare due risposte dalla
//! cronologia. Produce le stesse `RigaDiff` della vista diff di Git.

use crate::model::RigaDiff;

/// Calcola il diff fra due testi e restituisce le righe (ctx/add/rem).
pub fn diff_linee(vecchio: &str, nuovo: &str) -> Vec<RigaDiff> {
    let a: Vec<&str> = vecchio.lines().collect();
    let b: Vec<&str> = nuovo.lines().collect();
    let (n, m) = (a.len(), b.len());

    // Tabella LCS.
    let mut lcs = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }

    // Backtrack per costruire la sequenza di righe.
    let mut righe = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    let (mut nv, mut nn) = (1u32, 1u32); // numeri di riga 1-based
    while i < n && j < m {
        if a[i] == b[j] {
            righe.push(RigaDiff { tipo: "ctx".into(), testo: a[i].to_string(), vecchia: Some(nv), nuova: Some(nn) });
            i += 1; j += 1; nv += 1; nn += 1;
        } else if lcs[i + 1][j] >= lcs[i][j + 1] {
            righe.push(RigaDiff { tipo: "rem".into(), testo: a[i].to_string(), vecchia: Some(nv), nuova: None });
            i += 1; nv += 1;
        } else {
            righe.push(RigaDiff { tipo: "add".into(), testo: b[j].to_string(), vecchia: None, nuova: Some(nn) });
            j += 1; nn += 1;
        }
    }
    while i < n {
        righe.push(RigaDiff { tipo: "rem".into(), testo: a[i].to_string(), vecchia: Some(nv), nuova: None });
        i += 1; nv += 1;
    }
    while j < m {
        righe.push(RigaDiff { tipo: "add".into(), testo: b[j].to_string(), vecchia: None, nuova: Some(nn) });
        j += 1; nn += 1;
    }
    righe
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_semplice() {
        let righe = diff_linee("a\nb\nc", "a\nB\nc");
        let tipi: Vec<&str> = righe.iter().map(|r| r.tipo.as_str()).collect();
        // a uguale, b rimossa, B aggiunta, c uguale
        assert_eq!(tipi, vec!["ctx", "rem", "add", "ctx"]);
    }

    #[test]
    fn testi_uguali_solo_contesto() {
        let righe = diff_linee("x\ny", "x\ny");
        assert!(righe.iter().all(|r| r.tipo == "ctx"));
    }
}
