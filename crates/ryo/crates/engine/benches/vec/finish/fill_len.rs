//! redo-fills pass 2 (THROWAWAY, bead ryo-fmx3.1): a provider relation's row count in a file without its rows where the
//! provider knows it (`Provider::fill_len`), for the emptiness tests and the fills: a fill known to have no row is never
//! made (its table is the relation's empty one).

use super::{Exec, File, Provider, R, Run, Scope, Supplier, Table};

/// The relation's empty table in this run's catalog.
pub(super) fn empty_fill(run: &mut Run, rel: &str) -> R<Table> {
    let (_, names, tys) = run.catalog(rel).ok_or_else(|| format!("no relation {rel}"))?;
    // redo-alloc: through the pool's constructor (its columns kept and given back)
    Ok(Table::with(names, tys))
}

/// The fill of `rel` in file `file` by its supplier (the host's or the provider's), or its empty table where the supplier
/// says the file has no row of it.
pub(super) fn fill_or_empty(run: &mut Run, p: &mut dyn Provider, rel: &str, file: u32) -> R<Table> {
    if let Supplier::Host = run.supplier(rel) {
        if !sw!("VEC_NO_FILL_LEN") && super::host::host_len(rel) == Some(0) {
            return empty_fill(run, rel);
        }
        return super::host::host_fill(run, rel, file).unwrap_or_else(|| Err(format!("the host supplies no {rel}")));
    }
    let scope = Scope::File(file);
    if sw!("VEC_NO_FILL_LEN") {
        return p.fill(run, rel, scope);
    }
    match p.fill_len(run, rel, scope) {
        Some(0) => empty_fill(run, rel),
        _ => p.fill(run, rel, scope),
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
        if let Supplier::Host = run.supplier(rel)
            && let Some(n) = super::host::host_len(rel)
        {
            return Ok(n);
        }
        if let Some(n) = p.fill_len(run, rel, Scope::File(f.file)) {
            return Ok(n);
        }
        let t = fill_or_empty(run, p, rel, f.file)?;
        let n = t.n;
        f.fills.insert(rel.to_owned(), super::recycle::rc(t)); // redo-alloc: kept answers
        Ok(n)
    }
}
