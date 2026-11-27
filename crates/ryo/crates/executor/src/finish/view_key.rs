//! Bind a driver's key column once. Native position columns already hold the row ID.
use super::{File, Native, Run, Src, view_cols};

pub enum Key<'a> {
    Table(&'a [u64]),
    Position,
    Parent(&'a [u32]),
    Fact(crate::finish::fact_view::Column<'a>),
    Null,
    Cell(Native, usize),
}

impl<'a> Key<'a> {
    pub(crate) fn bind(f: &'a File<'_>, source: &'a Src, col: usize) -> Self {
        let (base, column) = match source {
            Src::Tab(table, _) => return Self::Table(&table.cols[col]),
            Src::View(view, _) => (view.base, view.map[col]),
            _ => unreachable!("resolved join driver"),
        };
        match (base, column) {
            (Native::Tree(_) | Native::Location, 0) | (Native::Child(_), 1) => Self::Position,
            (Native::Child(_), 0) => Self::Parent(&f.parent),
            (Native::Fact(i), c) => {
                f.fact(usize::from(i))
                    .map_or(Self::Null, |t| Self::Fact(t.columns().get(c)))
            }
            (native, c) => Self::Cell(native, c),
        }
    }

    #[inline]
    pub(crate) fn read(&self, f: &File<'_>, run: &mut Run, row: u32) -> u64 {
        if let Self::Table(keys) = self {
            return keys[row as usize];
        }
        match self {
            Self::Position => u64::from(row),
            Self::Parent(keys) => u64::from(keys[row as usize]),
            Self::Fact(keys) => keys.at(row as usize),
            Self::Cell(native, col) => view_cols::cell(f, run, *native, row, *col),
            Self::Null => super::NULL,
            Self::Table(_) => unreachable!(),
        }
    }
}
