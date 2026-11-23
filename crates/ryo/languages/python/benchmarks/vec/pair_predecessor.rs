//! Project the resolution provider's scheduled predecessor from its existing lookup and read rows.
use super::{CellKind, CellWord, ColumnFile, FactAnswer, FileWords, FilledColumns, Wanted};
use ryo::languages::{Relation, facts};
use ryo_relations::domain::{
    labels::types::NameRef, relations::service as catalog, terms::types::NameText,
};
use std::sync::OnceLock;

pub(crate) const COLUMNS: &[(&str, CellKind)] = &[
    ("reference", CellKind::Node),
    ("event", CellKind::Node),
    ("kind", CellKind::Symbol),
    ("namespace", CellKind::Symbol),
    ("scope", CellKind::Node),
    ("depth", CellKind::Number),
    ("ordinal", CellKind::Number),
    ("context", CellKind::Symbol),
    ("file", CellKind::File),
];

pub(crate) const LOOKUP: [&str; 8] = [
    "reference",
    "event",
    "kind",
    "scope",
    "lexical",
    "depth",
    "ordinal",
    "context",
];
pub(crate) const READ: [&str; 3] = ["reference", "scope", "context"];
pub(crate) const WORDS: [&str; 4] = ["deletion", "own", "routed", "none"];

// The resolver orders the lookup rows nearest namespace first. Preserve that
// order and the provider's explicit empty lookup. Both adapters use these rows.
pub(crate) fn project(
    lookups: impl Iterator<Item = [u64; 8]>,
    reads: impl ExactSizeIterator<Item = [u64; 3]>,
    [deletion, own, routed, none]: [u64; 4],
    file: u64,
    mut emit: impl FnMut([u64; 9]),
) {
    let mut seen = rustc_hash::FxHashSet::with_capacity_and_hasher(reads.len(), Default::default());
    for [reference, event, kind, scope, lexical, depth, ordinal, ctx] in lookups {
        if (ctx == deletion && depth != 0) || !seen.insert(reference) {
            continue;
        }
        let namespace = if lexical == scope { own } else { routed };
        emit([
            reference, event, kind, namespace, scope, depth, ordinal, ctx, file,
        ]);
    }
    let missing = super::super::finish::NULL;
    for [reference, scope, ctx] in reads {
        if seen.insert(reference) {
            emit([
                reference, missing, none, none, scope, missing, missing, ctx, file,
            ]);
        }
    }
}

pub(crate) fn positions<const N: usize, E>(
    names: [&str; N],
    mut find: impl FnMut(&str) -> Result<usize, E>,
) -> Result<[usize; N], E> {
    let mut columns = [0; N];
    for (column, name) in columns.iter_mut().zip(names) {
        *column = find(name)?;
    }
    Ok(columns)
}

struct Input<const N: usize> {
    relation: Relation,
    columns: [usize; N],
}
impl<const N: usize> Input<N> {
    fn new(name: &str, names: [&str; N]) -> Self {
        let relation = catalog::relation_named(NameRef(name)).expect("declared resolution input");
        let columns = catalog::columns(relation);
        let columns = names.map(|n| {
            columns
                .iter()
                .position(|c| catalog::column_name(*c).0 == n)
                .expect("declared resolution column")
        });
        Self { relation, columns }
    }
    fn complete(&self, table: &FilledColumns) -> bool {
        table.0.first().is_none_or(|first| {
            self.columns.iter().all(|&column| {
                table
                    .0
                    .get(column)
                    .is_some_and(|c| c.0.len() == first.0.len())
            })
        })
    }
    fn rows<'a>(
        &'a self,
        table: &'a FilledColumns,
    ) -> impl ExactSizeIterator<Item = [u64; N]> + 'a {
        let n = table.0.first().map_or(0, |c| c.0.len());
        (0..n).map(|r| self.columns.map(|c| table.0[c].0[r].0))
    }
}

pub(super) fn read(
    file: &mut ColumnFile<'_, '_>,
    number: usize,
    words: &mut FileWords,
    out: &mut FilledColumns,
) -> bool {
    static INPUTS: OnceLock<(Input<8>, Input<3>)> = OnceLock::new();
    let (lookup, read) = INPUTS.get_or_init(|| {
        (
            Input::new("binding-resolution", LOOKUP),
            Input::new("binding-read", READ),
        )
    });
    let mut lookups = FilledColumns::default();
    let mut reads = FilledColumns::default();
    if !matches!(
        file.fill(lookup.relation, Wanted::Whole, words, &mut lookups),
        FactAnswer::Answered(_)
    ) || !matches!(
        file.fill(read.relation, Wanted::Whole, words, &mut reads),
        FactAnswer::Answered(_)
    ) {
        return false;
    }
    if !lookup.complete(&lookups) || !read.complete(&reads) {
        return false;
    }
    let symbols = WORDS.map(|s| facts::intern(words, NameText(s)).0);
    project(
        lookup.rows(&lookups),
        read.rows(&reads),
        symbols,
        number as u64,
        |row| {
            for (col, value) in out.0.iter_mut().zip(row) {
                col.0.push(CellWord(value));
            }
        },
    );
    true
}
