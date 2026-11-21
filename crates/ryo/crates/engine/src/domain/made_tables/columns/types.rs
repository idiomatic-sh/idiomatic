//! The types of the `columns` part: a column kept in a table's rows, or the file's own, of which a table keeps no cell.

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
