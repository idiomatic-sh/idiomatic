//! redo-fills pass 2 (THROWAWAY, bead ryo-fmx3.1): a provider relation's row count in a file without its rows where the
//! provider knows it (`Provider::fill_len`), for the emptiness tests and the fills: a fill known to have no row is never
//! made (its table is the relation's empty one).

use super::{Exec, File, Provider, R, Run, Scope, Supplier, Table};

/// The relation's empty table in this run's catalog.
pub fn empty_fill(run: &mut Run, rel: &str) -> R<Table> {
    let (_, names, tys) = run.catalog(rel).ok_or_else(|| format!("no relation {rel}"))?;
    // redo-alloc: through the pool's constructor (its columns kept and given back)
    Ok(Table::with(names, tys))
}

/// The fill of `rel` in file `file` by its supplier (the host's or the provider's), or its empty table where the supplier
/// says the file has no row of it.
pub fn fill_or_empty(run: &mut Run, p: &mut dyn Provider, rel: &str, file: u32) -> R<std::rc::Rc<Table>> {
    if let Supplier::Host = run.supplier(rel) {
        if !sw!("VEC_NO_FILL_LEN") && super::host::host_len(rel) == Some(0) {
            return empty_fill(run, rel).map(super::recycle::rc);
        }
        return super::host::host_fill(run, rel, file).unwrap_or_else(|| Err(format!("the host supplies no {rel}"))).map(super::recycle::rc);
    }
    let scope = Scope::File(file);
    if sw!("VEC_NO_FILL_LEN") {
        return p.shared_fill(run, rel, scope);
    }
    match p.fill_len(run, rel, scope) {
        Some(0) => empty_fill(run, rel).map(super::recycle::rc),
        _ => p.shared_fill(run, rel, scope),
    }
}

impl Exec<'_> {
    /// How many rows the provider relation `rel` has in this file: the made fill's, else the provider's count without the
    /// rows, else the fill made (and kept, as `load_fact`/`base` keep it).
    pub fn fill_len(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, rel: &str) -> R<usize> {
        if let Some(t) = f.filled(rel) {
            return Ok(t.n);
        }
        if rel == super::host::CURRENT_FILE.0 && run.read_failure.as_ref().is_some_and(|(file, _, missing)| *file == f.file && *missing) {
            return Ok(0);
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
        f.fills.insert(rel.to_owned(), t); // redo-alloc: kept answers
        Ok(n)
    }
}
