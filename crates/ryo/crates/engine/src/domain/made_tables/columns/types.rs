//! The types of the `columns` part: a column kept in a table's rows, or the file's own, of which a table keeps no cell.

use crate::domain::code::types::{RelationNumber, Short};
use crate::domain::steps::types::MadeTableId;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// Where a column of a table lies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowColumn {
    /// In each row's cells, at this place.
    Kept(Count),
    /// Nowhere: it is the file's own column.
    File,
}

/// What a column of a table is, as its fill writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ColumnIs {
    /// The file's own: every pipeline writes the file there.
    TheFile,
    /// Written from a register or a constant.
    Written,
}

/// The file's cell as the one cell of a column that does not move from row to row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FileColumn(pub(crate) [CellWord; 1]);

/// Which cells a column that is held for the file lies in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HeldCells {
    /// The cells a made table keeps, row by row.
    Table(MadeTableId),
    /// The one cell a made table holds of the file: its file column, of which it keeps no cell a row.
    TableFile(MadeTableId),
    /// One column of a filled relation, cell after cell.
    Relation(RelationNumber, Short),
    /// None: the source keeps no cells (the node table's are made when read), or is not there for the file.
    Nothing,
}

/// A column of a made table or of a filled relation as its reader indexes it itself: the cell of the row numbered
/// `r` is `cells[first + r * stride]`. It holds places and no borrow, so it is kept while the store changes, and it
/// stays good for the rest of the file, as a `RowRun` does: a made table's cells do not move once it is made, and a
/// relation's rows are only added to. The file's own column of a made table is one cell with a stride of nought.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HeldColumn {
    /// The cells it lies in.
    pub(crate) cells: HeldCells,
    /// Where its first cell is.
    pub(crate) first: Count,
    /// How far apart its cells are.
    pub(crate) stride: Count,
}
