//! Integrazione con Git reale tramite il crate `git2` (binding di libgit2).
//! Il workspace è un repository git: salviamo le collection come file e qui
//! gestiamo stato, diff, commit e cronologia.

use crate::model::{Commit, FileModificato, RigaDiff, StatoRepo};
use git2::build::CheckoutBuilder;
use git2::{
    Cred, CredentialType, DiffFormat, DiffOptions, FetchOptions, PushOptions, RemoteCallbacks,
    Repository, Signature, Status, StatusOptions,
};
use std::path::Path;

/// Apre il repository del workspace, inizializzandolo se non esiste ancora.
pub fn assicura_repo(root: &Path) -> Result<Repository, git2::Error> {
    match Repository::open(root) {
        Ok(repo) => Ok(repo),
        Err(_) => Repository::init(root),
    }
}

/// Restituisce l'elenco dei file con modifiche non committate (M/A/D).
pub fn stato(root: &Path) -> Result<Vec<FileModificato>, git2::Error> {
    let repo = assicura_repo(root)?;
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);

    let mut risultato = Vec::new();
    for entry in repo.statuses(Some(&mut opts))?.iter() {
        let s = entry.status();
        // Ignora i file non modificati e quelli ignorati.
        if s.is_ignored() || s == Status::CURRENT {
            continue;
        }
        let stato = if s.intersects(Status::WT_NEW | Status::INDEX_NEW) {
            "A"
        } else if s.intersects(Status::WT_DELETED | Status::INDEX_DELETED) {
            "D"
        } else {
            "M"
        };
        if let Some(path) = entry.path() {
            risultato.push(FileModificato {
                file: path.to_string(),
                stato: stato.to_string(),
            });
        }
    }
    risultato.sort_by(|a, b| a.file.cmp(&b.file));
    Ok(risultato)
}

/// Calcola il diff (HEAD ↔ working dir) di un singolo file, riga per riga.
pub fn diff_file(root: &Path, file: &str) -> Result<Vec<RigaDiff>, git2::Error> {
    let repo = assicura_repo(root)?;

    let mut opts = DiffOptions::new();
    opts.pathspec(file).context_lines(3);

    // Albero di HEAD (None se il repo non ha ancora commit: tutto risulta "aggiunto").
    let tree = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_tree().ok());

    let diff = repo.diff_tree_to_workdir_with_index(tree.as_ref(), Some(&mut opts))?;

    let mut righe = Vec::new();
    diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
        match line.origin() {
            '+' | '-' | ' ' => {
                let tipo = match line.origin() {
                    '+' => "add",
                    '-' => "rem",
                    _ => "ctx",
                };
                let testo = String::from_utf8_lossy(line.content())
                    .trim_end_matches('\n')
                    .to_string();
                righe.push(RigaDiff {
                    tipo: tipo.to_string(),
                    testo,
                    vecchia: line.old_lineno(),
                    nuova: line.new_lineno(),
                });
            }
            // 'F' (file header) e 'H' (hunk header) vengono ignorati.
            _ => {}
        }
        true
    })?;

    Ok(righe)
}

/// Esegue lo stage dei file indicati e crea un commit. Restituisce lo SHA breve.
pub fn commit(root: &Path, messaggio: &str, files: &[String]) -> Result<String, git2::Error> {
    let repo = assicura_repo(root)?;
    let mut index = repo.index()?;

    // Stage di ogni file: se è stato eliminato dal disco usa remove_path.
    for f in files {
        let p = Path::new(f);
        if root.join(f).exists() {
            index.add_path(p)?;
        } else {
            index.remove_path(p)?;
        }
    }
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    // Firma: prova dalla config git dell'utente, altrimenti un default.
    let firma = repo
        .signature()
        .or_else(|_| Signature::now("Rustman", "rustman@local"))?;

    // Commit genitore (None per il primo commit del repo).
    let genitore = repo
        .head()
        .ok()
        .and_then(|h| h.target())
        .and_then(|oid| repo.find_commit(oid).ok());

    let genitori: Vec<&git2::Commit> = genitore.iter().collect();
    let oid = repo.commit(Some("HEAD"), &firma, &firma, messaggio, &tree, &genitori)?;

    Ok(oid.to_string()[..7].to_string())
}

/// Restituisce la cronologia dei commit, dal più recente, fino a `limite`.
pub fn log(root: &Path, limite: usize) -> Result<Vec<Commit>, git2::Error> {
    let repo = assicura_repo(root)?;
    let mut commits = Vec::new();

    // Se non c'è ancora HEAD (repo vuoto), restituisce lista vuota.
    if repo.head().is_err() {
        return Ok(commits);
    }

    let mut walk = repo.revwalk()?;
    walk.push_head()?;
    for oid in walk.take(limite) {
        let oid = oid?;
        let c = repo.find_commit(oid)?;
        let quando = formatta_tempo(c.time().seconds());
        commits.push(Commit {
            sha: oid.to_string()[..7].to_string(),
            messaggio: c.summary().unwrap_or("").to_string(),
            autore: c.author().name().unwrap_or("").to_string(),
            quando,
        });
    }
    Ok(commits)
}

/// Converte un timestamp Unix in una data leggibile (UTC, senza dipendenze esterne).
fn formatta_tempo(secondi: i64) -> String {
    // Calcolo civile da giorni dall'epoca (algoritmo di Howard Hinnant).
    let giorni = secondi.div_euclid(86_400);
    let resto = secondi.rem_euclid(86_400);
    let (ore, min) = (resto / 3600, (resto % 3600) / 60);

    let z = giorni + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let g = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let anno = if m <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02} {:02}:{:02}", anno, m, g, ore, min)
}

// ============================ Remote (pull/push) =============================

/// Informazioni sul repo: branch corrente, URL del remote e ahead/behind.
pub fn info(root: &Path) -> Result<StatoRepo, git2::Error> {
    let repo = assicura_repo(root)?;
    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(|s| s.to_string()));
    let remote = repo
        .find_remote("origin")
        .ok()
        .and_then(|r| r.url().map(|s| s.to_string()));
    let (ahead, behind) = ahead_behind(&repo).unwrap_or((0, 0));
    Ok(StatoRepo {
        branch,
        remote,
        ahead,
        behind,
    })
}

/// Calcola quanti commit il branch locale è avanti/indietro rispetto a origin.
fn ahead_behind(repo: &Repository) -> Option<(usize, usize)> {
    let head = repo.head().ok()?;
    let locale = head.target()?;
    let branch = head.shorthand()?;
    let upstream = repo
        .refname_to_id(&format!("refs/remotes/origin/{}", branch))
        .ok()?;
    repo.graph_ahead_behind(locale, upstream).ok()
}

/// Imposta (o aggiorna) l'URL del remote "origin".
pub fn imposta_remote(root: &Path, url: &str) -> Result<(), git2::Error> {
    let repo = assicura_repo(root)?;
    if repo.find_remote("origin").is_ok() {
        repo.remote_set_url("origin", url)?;
    } else {
        repo.remote("origin", url)?;
    }
    Ok(())
}

/// Callback con le credenziali del sistema (SSH agent o credential helper).
fn callbacks() -> RemoteCallbacks<'static> {
    let mut cb = RemoteCallbacks::new();
    cb.credentials(|url, username, allowed| {
        if allowed.contains(CredentialType::SSH_KEY) {
            return Cred::ssh_key_from_agent(username.unwrap_or("git"));
        }
        if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) {
            if let Ok(cfg) = git2::Config::open_default() {
                if let Ok(cred) = Cred::credential_helper(&cfg, url, username) {
                    return Ok(cred);
                }
            }
        }
        Cred::default()
    });
    cb
}

/// Esegue il pull (fetch di origin + fast-forward del branch corrente).
pub fn pull(root: &Path) -> Result<String, git2::Error> {
    let repo = assicura_repo(root)?;
    let branch = repo
        .head()?
        .shorthand()
        .ok_or_else(|| git2::Error::from_str("nessun branch corrente"))?
        .to_string();

    let mut remote = repo.find_remote("origin")?;
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(callbacks());
    remote.fetch(&[&branch], Some(&mut fo), None)?;

    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let (analisi, _) = repo.merge_analysis(&[&commit])?;

    if analisi.is_up_to_date() {
        return Ok("Già aggiornato.".into());
    }
    if analisi.is_fast_forward() {
        let refname = format!("refs/heads/{}", branch);
        match repo.find_reference(&refname) {
            Ok(mut r) => {
                r.set_target(commit.id(), "pull: fast-forward")?;
            }
            Err(_) => {
                repo.reference(&refname, commit.id(), true, "pull: branch iniziale")?;
            }
        }
        repo.set_head(&refname)?;
        repo.checkout_head(Some(CheckoutBuilder::default().force()))?;
        return Ok("Pull completato (fast-forward).".into());
    }
    Err(git2::Error::from_str(
        "Le modifiche divergono: serve un merge manuale (non supportato dall'app).",
    ))
}

/// Esegue il push del branch corrente su origin.
pub fn push(root: &Path) -> Result<String, git2::Error> {
    let repo = assicura_repo(root)?;
    let branch = repo
        .head()?
        .shorthand()
        .ok_or_else(|| git2::Error::from_str("nessun branch corrente"))?
        .to_string();

    let mut remote = repo.find_remote("origin")?;
    let mut po = PushOptions::new();
    po.remote_callbacks(callbacks());
    let refspec = format!("refs/heads/{0}:refs/heads/{0}", branch);
    remote.push(&[&refspec], Some(&mut po))?;
    Ok("Push completato.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_commit_e_log() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Crea un file e committalo.
        std::fs::write(root.join("a.json"), "{}\n").unwrap();
        let modifiche = stato(root).unwrap();
        assert_eq!(modifiche.len(), 1);
        assert_eq!(modifiche[0].stato, "A");

        let sha = commit(root, "primo commit", &["a.json".to_string()]).unwrap();
        assert_eq!(sha.len(), 7);

        // Dopo il commit non ci sono più modifiche.
        assert!(stato(root).unwrap().is_empty());

        let storia = log(root, 10).unwrap();
        assert_eq!(storia.len(), 1);
        assert_eq!(storia[0].messaggio, "primo commit");
    }
}
