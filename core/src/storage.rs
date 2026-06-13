//! Persistenza delle collection su file.
//!
//! Struttura su disco (la root del workspace è anche un repo git):
//! ```text
//! workspace/
//!   User APIs/            <- una collezione = una cartella di primo livello
//!     login.json          <- una richiesta = un file JSON formattato
//!     auth/               <- sottocartella (raggruppamento)
//!       refresh.json
//!   environments/         <- riservata agli ambienti (non è una collezione)
//! ```
//! Il JSON è "pretty" (indentato) così i diff di Git restano puliti.

use crate::model::{
    Albero, Auth, Catena, CatenaSuDisco, Collezione, Environment, EnvironmentSuDisco,
    EsportaCollezione, Nodo, NodoExport, Richiesta,
};
use std::fs;
use std::io;
use std::path::Path;

/// Nome della cartella riservata agli ambienti (non è una collezione).
const DIR_ENV: &str = "environments";
/// Nome della cartella riservata alle catene di run (non è una collezione).
const DIR_RUNS: &str = "runs";

/// Trasforma un nome leggibile in un nome file sicuro (es. "Update User" -> "update-user").
pub fn slug(nome: &str) -> String {
    let mut s = String::new();
    let mut trattino = false;
    for c in nome.trim().chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c.to_ascii_lowercase());
            trattino = false;
        } else if !trattino && !s.is_empty() {
            s.push('-');
            trattino = true;
        }
    }
    while s.ends_with('-') {
        s.pop();
    }
    if s.is_empty() {
        s.push_str("senza-nome");
    }
    s
}

/// Carica l'albero completo: ogni cartella di primo livello è una collezione.
pub fn carica_albero(root: &Path) -> io::Result<Albero> {
    let mut albero: Albero = Vec::new();
    if !root.exists() {
        return Ok(albero);
    }

    let mut cartelle: Vec<_> = fs::read_dir(root)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter(|e| !nome_nascosto(&e.file_name().to_string_lossy()))
        .collect();
    cartelle.sort_by_key(|e| e.file_name());

    for cartella in cartelle {
        let nome = cartella.file_name().to_string_lossy().to_string();
        let figli = leggi_cartella(root, &nome)?;
        albero.push(Collezione {
            nome: nome.clone(),
            dir: nome,
            figli,
        });
    }
    Ok(albero)
}

/// Legge ricorsivamente il contenuto di una cartella (relativa alla root):
/// prima le sottocartelle, poi le richieste (file .json).
fn leggi_cartella(root: &Path, rel: &str) -> io::Result<Vec<Nodo>> {
    let abs = root.join(rel);
    let mut entries: Vec<_> = fs::read_dir(&abs)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());

    let mut figli = Vec::new();

    // Sottocartelle (ricorsione).
    for e in &entries {
        let nome = e.file_name().to_string_lossy().to_string();
        if e.path().is_dir() && !nome_nascosto(&nome) {
            let sub_rel = format!("{}/{}", rel, nome);
            let sub = leggi_cartella(root, &sub_rel)?;
            figli.push(Nodo::Cartella {
                nome,
                dir: sub_rel,
                figli: sub,
            });
        }
    }
    // Richieste (file .json validi).
    for e in &entries {
        let p = e.path();
        if p.extension().map(|x| x == "json").unwrap_or(false) {
            let testo = fs::read_to_string(&p)?;
            if let Ok(richiesta) = serde_json::from_str::<Richiesta>(&testo) {
                let file = format!("{}/{}", rel, e.file_name().to_string_lossy());
                figli.push(Nodo::Richiesta {
                    nome: richiesta.nome.clone(),
                    file,
                    richiesta,
                });
            }
        }
    }
    Ok(figli)
}

/// Salva una richiesta in una cartella (anche annidata) e ne restituisce il percorso.
/// Se `file_precedente` ha un nome diverso (rinomina), il vecchio file viene rimosso.
pub fn salva_richiesta(
    root: &Path,
    dir: &str,
    file_precedente: Option<&str>,
    richiesta: &Richiesta,
) -> io::Result<String> {
    fs::create_dir_all(root.join(dir))?;
    let rel = format!("{}/{}.json", dir, slug(&richiesta.nome));

    let testo = serde_json::to_string_pretty(richiesta)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(root.join(&rel), testo)?;

    if let Some(prec) = file_precedente {
        if prec != rel {
            let _ = fs::remove_file(root.join(prec));
        }
    }
    Ok(rel)
}

/// Crea una cartella (collezione se `dir_genitore` è vuoto) e ne restituisce il percorso.
pub fn crea_cartella(root: &Path, dir_genitore: &str, nome: &str) -> io::Result<String> {
    let base = slug(nome);
    let rel = if dir_genitore.is_empty() {
        base
    } else {
        format!("{}/{}", dir_genitore, base)
    };
    fs::create_dir_all(root.join(&rel))?;
    Ok(rel)
}

/// Crea una collezione (cartella di primo livello).
pub fn crea_collezione(root: &Path, nome: &str) -> io::Result<String> {
    crea_cartella(root, "", nome)
}

/// Crea una nuova richiesta vuota (GET) dentro una cartella.
pub fn crea_richiesta(root: &Path, dir: &str, nome: &str) -> io::Result<String> {
    let richiesta = Richiesta {
        nome: nome.to_string(),
        metodo: "GET".to_string(),
        url: "https://".to_string(),
        headers: Vec::new(),
        params: Vec::new(),
        auth: Auth::default(),
        body: String::new(),
        tests: Vec::new(),
        pre_script: String::new(),
        post_script: String::new(),
    };
    salva_richiesta(root, dir, None, &richiesta)
}

/// Elimina un file (richiesta) dato il suo percorso relativo alla root.
pub fn elimina(root: &Path, file_relativo: &str) -> io::Result<()> {
    fs::remove_file(root.join(file_relativo))
}

/// Rinomina una cartella (mantenendo il genitore) e restituisce il nuovo percorso.
pub fn rinomina_cartella(root: &Path, dir: &str, nuovo_nome: &str) -> io::Result<String> {
    let genitore = Path::new(dir)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .filter(|s| !s.is_empty());
    let base = slug(nuovo_nome);
    let nuova = match genitore {
        Some(g) => format!("{}/{}", g, base),
        None => base,
    };
    if nuova != dir {
        fs::rename(root.join(dir), root.join(&nuova))?;
    }
    Ok(nuova)
}

/// Elimina un'intera cartella (e contenuto).
pub fn elimina_cartella(root: &Path, dir: &str) -> io::Result<()> {
    fs::remove_dir_all(root.join(dir))
}

/// True se la cartella va ignorata (nascosta o riservata).
fn nome_nascosto(nome: &str) -> bool {
    nome.starts_with('.') || nome == DIR_ENV || nome == DIR_RUNS
}

// ===================== Environments / variabili ==============================

/// Carica tutti gli ambienti dalla cartella `environments/`.
pub fn carica_environments(root: &Path) -> io::Result<Vec<EnvironmentSuDisco>> {
    let dir = root.join(DIR_ENV);
    let mut lista = Vec::new();
    if !dir.exists() {
        return Ok(lista);
    }
    let mut files: Vec<_> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
        .collect();
    files.sort_by_key(|e| e.file_name());

    for f in files {
        let testo = fs::read_to_string(f.path())?;
        if let Ok(environment) = serde_json::from_str::<Environment>(&testo) {
            let rel = format!("{}/{}", DIR_ENV, f.file_name().to_string_lossy());
            lista.push(EnvironmentSuDisco { file: rel, environment });
        }
    }
    Ok(lista)
}

/// Salva un ambiente e restituisce il suo percorso relativo (rinominando se serve).
pub fn salva_environment(
    root: &Path,
    file_precedente: Option<&str>,
    env: &Environment,
) -> io::Result<String> {
    let dir = root.join(DIR_ENV);
    fs::create_dir_all(&dir)?;
    let rel = format!("{}/{}.json", DIR_ENV, slug(&env.nome));
    let testo = serde_json::to_string_pretty(env)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(root.join(&rel), testo)?;
    if let Some(prec) = file_precedente {
        if prec != rel {
            let _ = fs::remove_file(root.join(prec));
        }
    }
    Ok(rel)
}

/// Elimina un ambiente dato il suo percorso relativo.
pub fn elimina_environment(root: &Path, file_relativo: &str) -> io::Result<()> {
    fs::remove_file(root.join(file_relativo))
}

// ===================== Run / catene di chiamate ==============================

/// Carica tutte le catene dalla cartella `runs/`.
pub fn carica_catene(root: &Path) -> io::Result<Vec<CatenaSuDisco>> {
    let dir = root.join(DIR_RUNS);
    let mut lista = Vec::new();
    if !dir.exists() {
        return Ok(lista);
    }
    let mut files: Vec<_> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
        .collect();
    files.sort_by_key(|e| e.file_name());

    for f in files {
        let testo = fs::read_to_string(f.path())?;
        if let Ok(catena) = serde_json::from_str::<Catena>(&testo) {
            let rel = format!("{}/{}", DIR_RUNS, f.file_name().to_string_lossy());
            lista.push(CatenaSuDisco { file: rel, catena });
        }
    }
    Ok(lista)
}

/// Salva una catena e restituisce il suo percorso relativo (rinominando se serve).
pub fn salva_catena(
    root: &Path,
    file_precedente: Option<&str>,
    catena: &Catena,
) -> io::Result<String> {
    let dir = root.join(DIR_RUNS);
    fs::create_dir_all(&dir)?;
    let rel = format!("{}/{}.json", DIR_RUNS, slug(&catena.nome));
    let testo = serde_json::to_string_pretty(catena)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(root.join(&rel), testo)?;
    if let Some(prec) = file_precedente {
        if prec != rel {
            let _ = fs::remove_file(root.join(prec));
        }
    }
    Ok(rel)
}

/// Elimina una catena dato il suo percorso relativo.
pub fn elimina_catena(root: &Path, file_relativo: &str) -> io::Result<()> {
    fs::remove_file(root.join(file_relativo))
}

// ===================== Import / Export =======================================

/// Esporta una collezione (con le sue sottocartelle) in una stringa JSON portabile.
pub fn esporta_collezione(root: &Path, dir: &str) -> io::Result<String> {
    let coll = carica_albero(root)?
        .into_iter()
        .find(|c| c.dir == dir)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "collezione non trovata"))?;

    let esporta = EsportaCollezione {
        rustman: 1,
        nome: coll.nome,
        figli: a_export(&coll.figli),
    };
    serde_json::to_string_pretty(&esporta)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Converte i nodi dell'albero nel formato di esportazione (senza percorsi).
fn a_export(nodi: &[Nodo]) -> Vec<NodoExport> {
    nodi
        .iter()
        .map(|n| match n {
            Nodo::Cartella { nome, figli, .. } => NodoExport::Cartella {
                nome: nome.clone(),
                figli: a_export(figli),
            },
            Nodo::Richiesta { richiesta, .. } => NodoExport::Richiesta {
                richiesta: richiesta.clone(),
            },
        })
        .collect()
}

/// Importa una collezione da una stringa JSON; restituisce la dir creata.
pub fn importa_collezione(root: &Path, contenuto: &str) -> io::Result<String> {
    let esporta: EsportaCollezione = serde_json::from_str(contenuto)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let dir = dir_unica(root, &slug(&esporta.nome));
    fs::create_dir_all(root.join(&dir))?;
    scrivi_export(root, &dir, &esporta.figli)?;
    Ok(dir)
}

/// Scrive ricorsivamente i nodi di un'esportazione dentro una cartella.
fn scrivi_export(root: &Path, dir: &str, figli: &[NodoExport]) -> io::Result<()> {
    for n in figli {
        match n {
            NodoExport::Richiesta { richiesta } => {
                salva_richiesta(root, dir, None, richiesta)?;
            }
            NodoExport::Cartella { nome, figli } => {
                let sub = crea_cartella(root, dir, nome)?;
                scrivi_export(root, &sub, figli)?;
            }
        }
    }
    Ok(())
}

/// Trova un nome di cartella libero, aggiungendo -2, -3, ... se già esiste.
fn dir_unica(root: &Path, base: &str) -> String {
    if !root.join(base).exists() {
        return base.to_string();
    }
    let mut i = 2;
    loop {
        let candidato = format!("{}-{}", base, i);
        if !root.join(&candidato).exists() {
            return candidato;
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn richiesta(nome: &str) -> Richiesta {
        Richiesta {
            nome: nome.into(),
            metodo: "GET".into(),
            url: "https://x".into(),
            headers: vec![],
            params: vec![],
            auth: Auth::default(),
            body: String::new(),
            tests: vec![],
            pre_script: String::new(),
            post_script: String::new(),
        }
    }

    /// Conta le richieste in un albero di nodi (ricorsivo).
    fn conta_richieste(figli: &[Nodo]) -> usize {
        figli
            .iter()
            .map(|n| match n {
                Nodo::Richiesta { .. } => 1,
                Nodo::Cartella { figli, .. } => conta_richieste(figli),
            })
            .sum()
    }

    #[test]
    fn slug_pulisce_i_nomi() {
        assert_eq!(slug("Update User"), "update-user");
        assert_eq!(slug("  GET /profile!! "), "get-profile");
        assert_eq!(slug(""), "senza-nome");
    }

    #[test]
    fn cartelle_annidate() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        crea_collezione(root, "Test").unwrap();
        salva_richiesta(root, "test", None, &richiesta("Ping")).unwrap();
        let sub = crea_cartella(root, "test", "Auth").unwrap();
        assert_eq!(sub, "test/auth");
        salva_richiesta(root, "test/auth", None, &richiesta("Login")).unwrap();

        let albero = carica_albero(root).unwrap();
        assert_eq!(albero.len(), 1);
        // 1 richiesta diretta + 1 dentro la sottocartella
        assert_eq!(conta_richieste(&albero[0].figli), 2);
        // c'è una cartella "auth" tra i figli
        let ha_cartella = albero[0]
            .figli
            .iter()
            .any(|n| matches!(n, Nodo::Cartella { dir, .. } if dir == "test/auth"));
        assert!(ha_cartella);
    }

    #[test]
    fn esporta_e_importa_con_sottocartelle() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        crea_collezione(root, "User APIs").unwrap();
        salva_richiesta(root, "user-apis", None, &richiesta("Login")).unwrap();
        crea_cartella(root, "user-apis", "Admin").unwrap();
        salva_richiesta(root, "user-apis/admin", None, &richiesta("Ban")).unwrap();

        let json = esporta_collezione(root, "user-apis").unwrap();
        let nuova_dir = importa_collezione(root, &json).unwrap();
        assert_ne!(nuova_dir, "user-apis");

        let albero = carica_albero(root).unwrap();
        assert_eq!(albero.len(), 2);
        let importata = albero.iter().find(|c| c.dir == nuova_dir).unwrap();
        assert_eq!(conta_richieste(&importata.figli), 2); // Login + Ban
    }

    #[test]
    fn catene_roundtrip() {
        use crate::model::{Catena, Passo};
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        let c = Catena {
            nome: "Flusso Login".into(),
            passi: vec![Passo { file: "test/login.json".into() }],
        };
        let rel = salva_catena(root, None, &c).unwrap();
        assert_eq!(rel, "runs/flusso-login.json");

        let catene = carica_catene(root).unwrap();
        assert_eq!(catene.len(), 1);
        assert_eq!(catene[0].catena.passi.len(), 1);

        // La cartella runs/ non deve comparire tra le collezioni.
        assert!(carica_albero(root).unwrap().is_empty());
    }
}
