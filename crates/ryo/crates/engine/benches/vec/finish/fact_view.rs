//! A native fact operator borrows only the column it reads. Provider node
//! positions and words remain in their file's namespace until that read.
use super::{NULL, Table, Ty};
use ryo::languages::{CellKind, CellWord, FilledColumn};
use std::cell::{Cell, OnceCell};

/// The executor's words of a lent relation's columns, translated once for the columns a consumer reads often.
/// A column read about as often as it has rows stops paying a translation per cell.
pub(crate) struct ColumnCache {
    cells: Vec<OnceCell<Vec<u64>>>,
    reads: Vec<Cell<u32>>,
}

impl ColumnCache {
    pub(crate) fn new(columns: usize) -> Self {
        Self { cells: (0..columns).map(|_| OnceCell::new()).collect(), reads: (0..columns).map(|_| Cell::new(0)).collect() }
    }
}

#[derive(Clone, Copy)]
pub(super) enum Column<'a> {
    Owned(&'a [u64]),
    Numbers(&'a [CellWord]),
    Nodes(&'a [CellWord]),
    Words(&'a [CellWord], &'a [u32], &'a rustc_hash::FxHashMap<u32,u32>, u32),
}

impl Column<'_> {
    #[inline]
    pub(super) fn at(self, row: usize) -> u64 {
        match self {
            Self::Owned(c) => c[row],
            Self::Numbers(c) => c[row].0,
            Self::Nodes(c) => {
                let value = c[row].0;
                if value == NULL { NULL } else { u64::from(value as u32) }
            }
            Self::Words(c, words, public, absent) => {
                let value = c[row].0;
                u64::from(words.get(value as usize).copied().or_else(|| u32::try_from(value).ok().and_then(|v| public.get(&v).copied())).unwrap_or(absent))
            }
        }
    }

    /// The first `n` cells as words, the representation matched once for the whole column.
    pub(super) fn extend_into(self, out: &mut Vec<u64>, n: usize) {
        match self {
            Self::Owned(c) => out.extend_from_slice(&c[..n]),
            Self::Numbers(c) => out.extend(c[..n].iter().map(|cell| cell.0)),
            Self::Nodes(c) => out.extend(c[..n].iter().map(|cell| if cell.0 == NULL { NULL } else { u64::from(cell.0 as u32) })),
            Self::Words(c, words, public, absent) => out.extend(c[..n].iter().map(|cell| {
                let value = cell.0;
                u64::from(words.get(value as usize).copied().or_else(|| u32::try_from(value).ok().and_then(|v| public.get(&v).copied())).unwrap_or(absent))
            })),
        }
    }

    /// Every cell of the column in the executor's words, one typed pass over the provider's storage.
    fn translated(self) -> Vec<u64> {
        let mut out = Vec::new();
        self.extend_into(&mut out, self.len());
        out
    }

    pub(super) fn len(self) -> usize {
        match self {
            Self::Owned(c) => c.len(),
            Self::Numbers(c) | Self::Nodes(c) | Self::Words(c, _, _, _) => c.len(),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Columns<'a> {
    Owned(&'a [Vec<u64>]),
    Lent(&'a [FilledColumn], &'a [CellKind], &'a [u32], &'a rustc_hash::FxHashMap<u32,u32>, u32, &'a ColumnCache),
}

impl<'a> Columns<'a> {
    /// One cell. A column read cell by cell is translated once, in a typed pass, after enough reads to pay for it.
    #[inline]
    pub(super) fn cell(self, column: usize, row: usize) -> u64 {
        let cache = match self {
            Self::Owned(c) => return c[column][row],
            Self::Lent(_, _, _, _, _, cache) => cache,
        };
        if let Some(cells) = cache.cells[column].get() {
            return cells[row];
        }
        let lent = self.raw(column);
        if matches!(lent, Column::Nodes(_) | Column::Words(..)) {
            let asked = cache.reads[column].get().saturating_add(1);
            cache.reads[column].set(asked);
            if (asked as usize).saturating_mul(8) >= lent.len() {
                return cache.cells[column].get_or_init(|| lent.translated())[row];
            }
        }
        lent.at(row)
    }

    #[inline]
    fn raw(self, column: usize) -> Column<'a> {
        match self {
            Self::Owned(c) => Column::Owned(&c[column]),
            Self::Lent(cols, kinds, words, public, absent, _) => {
                let cells = cols.get(column).map_or(&[][..], |c| c.0.as_slice());
                match kinds[column] {
                    CellKind::Node => Column::Nodes(cells),
                    CellKind::Symbol => Column::Words(cells, words, public, absent),
                    _ => Column::Numbers(cells),
                }
            }
        }
    }

    #[inline]
    pub(super) fn get(self, column: usize) -> Column<'a> {
        match self {
            Self::Owned(c) => Column::Owned(&c[column]),
            Self::Lent(cols, kinds, words, public, absent, cache) => {
                let cells = cols.get(column).map_or(&[][..], |c| c.0.as_slice());
                cached(cache, column, match kinds[column] {
                    CellKind::Node => Column::Nodes(cells),
                    CellKind::Symbol => Column::Words(cells, words, public, absent),
                    _ => Column::Numbers(cells),
                })
            }
        }
    }
}

/// The column itself, or its translation once it has been asked for about as often as it has rows.
fn cached<'a>(cache: &'a ColumnCache, column: usize, lent: Column<'a>) -> Column<'a> {
    if !matches!(lent, Column::Nodes(_) | Column::Words(..)) {
        return lent;
    }
    if let Some(cells) = cache.cells[column].get() {
        return Column::Owned(cells);
    }
    let asked = cache.reads[column].get().saturating_add(1);
    cache.reads[column].set(asked);
    let rows = lent.len();
    if (asked as usize).saturating_mul(4) < rows {
        return lent;
    }
    Column::Owned(cache.cells[column].get_or_init(|| lent.translated()))
}

#[derive(Clone, Copy)]
pub(crate) enum Fact<'a> {
    Owned(&'a Table),
    Lent { rows: usize, columns: Columns<'a>, kinds: &'a [CellKind] },
}

impl<'a> Fact<'a> {
    pub(super) fn len(self) -> usize {
        match self { Self::Owned(t) => t.n, Self::Lent { rows, .. } => rows }
    }
    #[inline]
    pub(super) fn cell(self, column: usize, row: usize) -> u64 {
        self.columns().cell(column, row)
    }
    pub(super) fn columns(self) -> Columns<'a> {
        match self { Self::Owned(t) => Columns::Owned(&t.cols), Self::Lent { columns, .. } => columns }
    }
    pub(super) fn ty(self, column: usize) -> Ty {
        match self { Self::Owned(t) => t.tys[column], Self::Lent { kinds, .. } => super::ty_of(kinds[column]) }
    }
}

/// The file supplies a loan only for a complete population in this version.
pub(crate) trait FileFacts {
    fn bind(&self, relation: ryo::languages::Relation, run: &mut super::Run) -> Result<Option<Fact<'_>>, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_cells_are_not_file_dictionary_offsets() {
        use ryo_relations::domain::terms::{service::unknown_symbol, types::UnknownReason};
        let reason = unknown_symbol(UnknownReason(23)).0;
        let mut public = rustc_hash::FxHashMap::default();
        public.insert(reason, 214);
        let cells = [CellWord(u64::from(reason)), CellWord(u64::from(ryo::languages::NO_SYMBOL.0))];
        let column = Column::Words(&cells, &[], &public, 12);
        assert_eq!(column.at(0), 214);
        assert_eq!(column.at(1), 12);
    }

    #[test]
    fn file_columns_match_owned_cells_in_the_executor_namespace() {
        let local = [
            FilledColumn(vec![CellWord((3 << 32) | 8), CellWord(NULL)]),
            FilledColumn(vec![CellWord(1), CellWord(u64::from(ryo::languages::NO_SYMBOL.0))]),
            FilledColumn(vec![CellWord(0), CellWord(42)]),
        ];
        let kinds = [CellKind::Node, CellKind::Symbol, CellKind::Number];
        let symbols = [33, 44];
        let public = rustc_hash::FxHashMap::default();
        let file = Columns::Lent(&local, &kinds, &symbols, &public, 55, &ColumnCache::new(3));
        let expected = [vec![8, NULL], vec![44, 55], vec![0, 42]];
        let owned = Columns::Owned(&expected);
        for col in 0..kinds.len() {
            let left = file.get(col);
            let right = owned.get(col);
            assert_eq!(left.len(), right.len());
            for row in 0..right.len() {
                assert_eq!(left.at(row), right.at(row));
            }
        }
    }
}
