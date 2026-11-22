//! redo-fills pass 2 (THROWAWAY, bead ryo-fmx3.1): a provider relation's row count in a file without its rows where the
//! provider knows it (`Provider::fill_len`), for the emptiness tests and the fills: a fill known to have no row is never
//! made (its table is the relation's empty one).

use super::{Exec, File, Provider, R, Run, Table};

/// The relation's empty table in this run's catalog.
pub(super) fn empty_fill(run: &mut Run, rel: &str) -> R<Table> {
    let (_, names, tys) = run.catalog(rel).ok_or_else(|| format!("no relation {rel}"))?;
    // redo-alloc: through the pool's constructor (its columns kept and given back)
    Ok(Table::with(names, tys))
}

/// The provider's fill of `rel`, or its empty table where the provider says the file has no row of it.
pub(super) fn fill_or_empty(run: &mut Run, p: &mut dyn Provider, rel: &str) -> R<Table> {
    if sw!("VEC_NO_FILL_LEN") {
        return p.fill(run, rel);
    }
    match p.fill_len(run, rel) {
        Some(0) => empty_fill(run, rel),
        _ => p.fill(run, rel),
    }
}

impl Exec<'_> {
    /// How many rows the provider relation `rel` has in this file: the made fill's, else the provider's count without the
    /// rows, else the fill made (and kept, as `load_fact`/`base` keep it).
    #[expect(dead_code, reason = "the emptiness leaves' entry (claude/redo-empty), joined by the merge builder")]
    pub(super) fn fill_len(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, rel: &str) -> R<usize> {
        if let Some(t) = f.fills.get(rel) {
            return Ok(t.n);
        }
        if let Some(n) = p.fill_len(run, rel) {
            return Ok(n);
        }
        let t = p.fill(run, rel)?;
        let n = t.n;
        f.fills.insert(rel.to_owned(), super::recycle::rc(t)); // redo-alloc: kept answers
        Ok(n)
    }
}
