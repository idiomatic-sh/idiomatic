//! redo-general (THROWAWAY, bead ryo-fmx3.1): the rows of a relation the provider answers for a PROJECT (`Supplier::Project`):
//! rows that relate the run's files to each other or to the project's entries. The executor asks them with
//! `Scope::Files` and keeps the answer for the run, by the file each row names (the relation's first file column); a
//! step of a file reads the rows that name that file. How wide each ask is, the provider declares (`ProjectAsk`).

use super::{File, FxHashMap, Provider, R, Run, Scope, Table, Ty};

/// How the provider answers a project relation, as it declares it.
#[derive(Clone, Copy, Debug)]
pub(in super::super) enum ProjectAsk {
    /// From the project alone (its layout, never a file's content): asked once a run, for all the run's files.
    Run,
    /// From each file's own content as the provider reads it, with the project's entries: asked for the files read and
    /// not yet answered. One file at a time (this executor), that is the file being run: the ask a project phase after
    /// the visit would make once a run is the contract's missing piece (`NOTES.md`, "one file at a time").
    EachFile,
}

/// A project relation's answers so far: the rows, by the file they name, and the files answered (an `EachFile` relation
/// keeps the current file's rows only).
#[derive(Default)]
pub(in super::super) struct Answered {
    table: Option<Table>,
    by_file: FxHashMap<u64, Vec<u32>>,
    asked: Vec<bool>,
    /// The provider's answers: asks, the files their scopes named, rows.
    asks: [u64; 3],
}

/// One line a project relation: the provider's asks, the files their scopes named, the rows answered.
pub(in super::super) fn report(run: &Run) -> Vec<String> {
    let mut v: Vec<String> = run.project.iter().map(|(rel, a)| format!("PROJECT {rel}: asks {} scope files {} rows {}", a.asks[0], a.asks[1], a.asks[2])).collect();
    v.sort();
    v
}

/// The rows of project relation `rel` that name file `f`, asked of the provider when `f` is not answered yet.
pub(super) fn rows(f: &File<'_>, run: &mut Run, p: &mut dyn Provider, rel: &str, ask: ProjectAsk) -> R<Table> {
    for_file(f.file, run, rel, ask, |run, files| p.fill(run, rel, Scope::Files(files)))
}

/// The current file's slice of the shared project answer, for either phase.
pub(in super::super) fn for_file(file: u32, run: &mut Run, rel: &str, ask: ProjectAsk, fill: impl FnOnce(&mut Run, &[u32]) -> R<Table>) -> R<Table> {
    let files = run.paths.len();
    let mut a = run.project.remove(rel).unwrap_or_default();
    a.asked.resize(files, false);
    if !a.asked[file as usize] {
        let scope: Vec<u32> = match ask {
            ProjectAsk::Run => (0..files as u32).collect(),
            ProjectAsk::EachFile => vec![file],
        };
        let t = fill(run, &scope);
        let t = match t {
            Ok(t) => t,
            Err(e) => {
                run.project.insert(rel.to_owned(), a);
                return Err(e);
            }
        };
        let key = t.tys.iter().position(|ty| *ty == Ty::File).ok_or_else(|| format!("{rel}: a project relation names no file"))?;
        // an answer for the file being run is read by that file's steps only: the last file's rows are not kept
        if let ProjectAsk::EachFile = ask {
            a.table = None;
            a.by_file.clear();
        }
        let base = a.table.as_ref().map_or(0, |x| x.n) as u32;
        for r in 0..t.n {
            a.by_file.entry(t.cols[key][r]).or_default().push(base + r as u32);
        }
        a.asks[0] += 1;
        a.asks[1] += scope.len() as u64;
        a.asks[2] += t.n as u64;
        for s in scope {
            a.asked[s as usize] = true;
        }
        match &mut a.table {
            Some(all) => {
                for (c, col) in t.cols.iter().enumerate() {
                    all.cols[c].extend_from_slice(col);
                }
                all.n += t.n;
            }
            None => a.table = Some(t),
        }
    }
    let all = a.table.as_ref().ok_or_else(|| format!("{rel}: no answer"))?;
    let mine = a.by_file.get(&u64::from(file)).map_or(&[][..], Vec::as_slice);
    let mut out = Table::with_rows(all.names.clone(), all.tys.clone(), mine.len());
    for (c, col) in out.cols.iter_mut().enumerate() {
        col.extend(mine.iter().map(|r| all.cols[c][*r as usize]));
    }
    out.n = mine.len();
    run.project.insert(rel.to_owned(), a);
    Ok(out)
}
