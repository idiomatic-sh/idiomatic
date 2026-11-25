//! A native fact operator borrows only the column it reads. Provider node
//! positions and words remain in their file's namespace until that read.
use super::{NULL, Table, Ty};
use ryo::languages::{CellKind, CellWord, FilledColumn};

#[derive(Clone, Copy)]
pub(super) enum Column<'a> {
    Owned(&'a [u64]),
    Numbers(&'a [CellWord]),
    Nodes(&'a [CellWord]),
    Words(&'a [CellWord], &'a [u32], u32),
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
            Self::Words(c, words, absent) => {
                let value = c[row].0;
                u64::from(if value == u64::from(ryo::languages::NO_SYMBOL.0) { absent } else { words[value as usize] })
            }
        }
    }

    pub(super) fn len(self) -> usize {
        match self {
            Self::Owned(c) => c.len(),
            Self::Numbers(c) | Self::Nodes(c) | Self::Words(c, _, _) => c.len(),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Columns<'a> {
    Owned(&'a [Vec<u64>]),
    Lent(&'a [FilledColumn], &'a [CellKind], &'a [u32], u32),
}

impl<'a> Columns<'a> {
    #[inline]
    pub(super) fn get(self, column: usize) -> Column<'a> {
        match self {
            Self::Owned(c) => Column::Owned(&c[column]),
            Self::Lent(cols, kinds, words, absent) => {
                let cells = cols.get(column).map_or(&[][..], |c| c.0.as_slice());
                match kinds[column] {
                    CellKind::Node => Column::Nodes(cells),
                    CellKind::Symbol => Column::Words(cells, words, absent),
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
    fn bind(&self, relation: ryo::languages::Relation, run: &mut super::Run) -> Result<bool, String>;
    fn relation(&self, relation: ryo::languages::Relation) -> Option<Fact<'_>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_columns_match_owned_cells_in_the_executor_namespace() {
        let local = [
            FilledColumn(vec![CellWord((3 << 32) | 8), CellWord(NULL)]),
            FilledColumn(vec![CellWord(1), CellWord(u64::from(ryo::languages::NO_SYMBOL.0))]),
            FilledColumn(vec![CellWord(0), CellWord(42)]),
        ];
        let kinds = [CellKind::Node, CellKind::Symbol, CellKind::Number];
        let symbols = [33, 44];
        let file = Columns::Lent(&local, &kinds, &symbols, 55);
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
