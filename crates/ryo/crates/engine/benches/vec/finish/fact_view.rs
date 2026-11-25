//! A native fact operator borrows only the column it reads. Provider node
//! positions and words remain in their file's namespace until that read.
use super::{NULL, Table, Ty};
use ryo::languages::{CellKind, CellWord, FilledColumn};

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
    Lent(&'a [FilledColumn], &'a [CellKind], &'a [u32], &'a rustc_hash::FxHashMap<u32,u32>, u32),
}

impl<'a> Columns<'a> {
    #[inline]
    pub(super) fn get(self, column: usize) -> Column<'a> {
        match self {
            Self::Owned(c) => Column::Owned(&c[column]),
            Self::Lent(cols, kinds, words, public, absent) => {
                let cells = cols.get(column).map_or(&[][..], |c| c.0.as_slice());
                match kinds[column] {
                    CellKind::Node => Column::Nodes(cells),
                    CellKind::Symbol => Column::Words(cells, words, public, absent),
                    _ => Column::Numbers(cells),
                }
            }
        }
    }
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
        let file = Columns::Lent(&local, &kinds, &symbols, &public, 55);
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
