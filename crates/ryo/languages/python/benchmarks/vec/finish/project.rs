//! redo-general (THROWAWAY, bead ryo-fmx3.1): the rows of a relation the provider answers for a PROJECT (`Supplier::Project`):
//! rows that relate the run's files to each other or to the project's entries. The executor asks them with
//! `Scope::Files` and keeps the answer for the run, by the file each row names (the relation's first file column); a
//! step of a file reads the rows that name that file. How wide each ask is, the provider declares (`ProjectAsk`).

use super::{File, FxHashMap, R, Run, Table, Ty};

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

/// A project relation's answers so far: the rows, by the file they name, and the files answered.
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

/// Whether the cached project answer covers this file.
pub(super) fn covers(run: &Run, rel: &str, file: u32) -> bool {
    run.project.get(rel).and_then(|a| a.asked.get(file as usize)).copied().unwrap_or(false)
}

/// Index a project answer by its declared file column. The whole-run table
/// stays here so subsequent files can read their share without another call.
pub(super) fn remember(run: &mut Run, rel: &str, files: &[u32], t: Table) -> R<()> {
    let key = t.tys.iter().position(|ty| *ty == Ty::File).ok_or_else(|| format!("{rel}: a project relation names no file"))?;
    let a = run.project.entry(rel.to_owned()).or_default();
    let base = a.table.as_ref().map_or(0, |x| x.n) as u32;
    for (row, file) in t.cols[key].iter().enumerate() {
        a.by_file.entry(*file).or_default().push(base + row as u32);
    }
    a.asks[0] += 1;
    a.asks[1] += files.len() as u64;
    a.asks[2] += t.n as u64;
    a.asked.resize(run.paths.len(), false);
    for &file in files {
        a.asked[file as usize] = true;
    }
    match &mut a.table {
        Some(all) => {
            for (dest, src) in all.cols.iter_mut().zip(&t.cols) {
                dest.extend_from_slice(src);
            }
            all.n += t.n;
        }
        None => a.table = Some(t),
    }
    Ok(())
}

pub(super) fn rows(f: &File<'_>, run: &Run, rel: &str) -> R<Table> {
    let a = run.project.get(rel).ok_or_else(|| format!("{rel}: no project answer"))?;
    let all = a.table.as_ref().ok_or_else(|| format!("{rel}: no answer"))?;
    let mine = a.by_file.get(&u64::from(f.file)).map_or(&[][..], Vec::as_slice);
    let mut out = Table::with_rows(all.names.clone(), all.tys.clone(), mine.len());
    for (c, col) in out.cols.iter_mut().enumerate() {
        col.extend(mine.iter().map(|r| all.cols[c][*r as usize]));
    }
    out.n = mine.len();
    Ok(out)
}
