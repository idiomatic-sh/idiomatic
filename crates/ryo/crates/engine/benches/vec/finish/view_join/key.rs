//! Bind a driver's key column once. Native position columns already hold the row ID.
use super::{File, Native, Run, Src, fill_prof, view_cols};

pub(super) enum Key<'a> {
    Table(&'a [u64]),
    Position,
    Parent(&'a [u32]),
    Fact(&'a [u64]),
    Null,
    Cell(Native, usize),
}

impl<'a> Key<'a> {
    pub(super) fn bind(f: &'a File<'_>, source: &'a Src, col: usize) -> Self {
        let (base, column) = match source {
            Src::Tab(table, _) => return Self::Table(&table.cols[col]),
            Src::View(view, _) => (view.base, view.map[col]),
            _ => unreachable!("resolved join driver"),
        };
        match (base, column) {
            (Native::Tree | Native::Location, 0) | (Native::Child, 1) => Self::Position,
            (Native::Child, 0) => Self::Parent(&f.parent),
            (Native::Fact(i), c) => {
                if fill_prof::counting() {
                    f.mark_read(i);
                }
                f.facts[usize::from(i)]
                    .as_ref()
                    .map_or(Self::Null, |t| Self::Fact(&t.cols[c]))
            }
            (native, c) => Self::Cell(native, c),
        }
    }

    #[inline]
    pub(super) fn read(&self, f: &File<'_>, run: &mut Run, row: u32) -> u64 {
        if let Self::Table(keys) = self {
            return keys[row as usize];
        }
        fill_prof::read(6);
        match self {
            Self::Position => u64::from(row),
            Self::Parent(keys) => u64::from(keys[row as usize]),
            Self::Fact(keys) => keys[row as usize],
            Self::Cell(native, col) => view_cols::cell(f, run, *native, row, *col),
            Self::Null => super::NULL,
            Self::Table(_) => unreachable!(),
        }
    }
}
