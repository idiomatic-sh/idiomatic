//! The services of the `reading` part: the one lookup the engine reads a table by, whatever its layout: whether a key has
//! a row, the rows of a key, every row, and the cells of a column.

use crate::domain::fact_tables::keying::service::{key_code, present_at, row_number};
use crate::domain::fact_tables::types::{
    FactCells, FactKey, FactRow, FactRowList, FactRowRun, FactTable, IndexOrder, KeyPresence,
    TableLayout,
};
use ryo_relations::domain::relations::types::{Column, Count};

/// Whether `key` has a row in `column`: one bit read, in either layout. A column the relation is not keyed by, or a key
/// of another space, answers `Absent`.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::reading::service::presence;
/// use ryo_language::domain::fact_tables::types::{FactKey, FactTable, KeyPresence};
/// use ryo_relations::domain::relations::collections::relations::comment::item::CommentColumn;
/// use ryo_relations::domain::relations::types::{Column, Position};
///
/// let id = Column::Comment(CommentColumn::Id);
/// assert_eq!(presence(&FactTable::default(), id, FactKey::Node(Position(0))), KeyPresence::Absent);
/// ```
#[must_use]
pub fn presence(table: &FactTable, column: Column, key: FactKey) -> KeyPresence {
    let found = table
        .body
        .indexes
        .iter()
        .find(|index| index.column == column)
        .and_then(|index| key_code(index.space, key).and_then(|code| present_at(index, code)));
    match found {
        Some(_) => KeyPresence::Held,
        None => KeyPresence::Absent,
    }
}

/// The rows whose `column` holds `key`, in the order they were written: none when it has none. The same call for one
/// row at most a key and for a list a key; a column the relation is not keyed by answers none.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::reading::service::rows_of;
/// use ryo_language::domain::fact_tables::types::{FactKey, FactTable};
/// use ryo_relations::domain::relations::collections::relations::comment::item::CommentColumn;
/// use ryo_relations::domain::relations::types::{Column, Position};
///
/// let id = Column::Comment(CommentColumn::Id);
/// assert_eq!(rows_of(&FactTable::default(), id, FactKey::Node(Position(0))).into_iter().count(), 0);
/// ```
#[must_use]
pub fn rows_of(table: &FactTable, column: Column, key: FactKey) -> FactRowList<'_> {
    let none = FactRowList::Run(FactRowRun::default());
    let Some(index) = table
        .body
        .indexes
        .iter()
        .find(|index| index.column == column)
    else {
        return none;
    };
    let Some(rank) = key_code(index.space, key).and_then(|code| present_at(index, code)) else {
        return none;
    };
    let at = usize::try_from(rank.0).unwrap_or(usize::MAX);
    let run = match index.layout {
        TableLayout::PerKey => FactRowRun {
            first: FactRow(rank.0),
            end: FactRow(rank.0.saturating_add(1)),
        },
        TableLayout::Listed => {
            match (
                index.offsets.get(at),
                index.offsets.get(at.saturating_add(1)),
            ) {
                (Some(first), Some(end)) => FactRowRun {
                    first: *first,
                    end: *end,
                },
                (Some(_) | None, _) => return none,
            }
        }
    };
    match index.order {
        IndexOrder::Keyed => FactRowList::Run(run),
        IndexOrder::Other => {
            let from = usize::try_from(run.first.0).unwrap_or(usize::MAX);
            let to = usize::try_from(run.end.0).unwrap_or(usize::MAX);
            index.listed.get(from..to).map_or(none, FactRowList::Listed)
        }
    }
}

/// Every row of the table, in the order of the key it is filled by (written order for a relation filled whole).
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::reading::service::every_row;
/// use ryo_language::domain::fact_tables::types::{FactRowRun, FactTable};
///
/// assert_eq!(every_row(&FactTable::default()), FactRowRun::default());
/// ```
#[must_use]
pub fn every_row(table: &FactTable) -> FactRowRun {
    FactRowRun {
        first: FactRow(0),
        end: row_number(table.body.rows),
    }
}

/// The cells of one column, one a row, indexed by a row's number: none for a column of another relation.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::reading::service::cells_of;
/// use ryo_language::domain::fact_tables::types::FactTable;
/// use ryo_relations::domain::relations::collections::relations::comment::item::CommentColumn;
/// use ryo_relations::domain::relations::types::{Column, Count};
///
/// assert!(cells_of(&FactTable::default(), Column::Comment(CommentColumn::File)).0.is_empty());
/// ```
#[must_use]
pub fn cells_of(table: &FactTable, column: Column) -> FactCells<'_> {
    let body = &table.body;
    body.named
        .iter()
        .position(|each| *each == column)
        .and_then(|place| body.columns.get(place))
        .map_or(FactCells(&[]), |cells| FactCells(cells))
}

/// The cells of the column at `place` among the relation's columns in the catalog's order, one a row: none past the last
/// column. The read of a column whose place the reader found once.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::reading::service::cells_at;
/// use ryo_language::domain::fact_tables::types::FactTable;
/// use ryo_relations::domain::relations::types::Count;
///
/// assert!(cells_at(&FactTable::default(), Count(0)).0.is_empty());
/// ```
#[inline]
#[must_use]
pub fn cells_at(table: &FactTable, place: Count) -> FactCells<'_> {
    table
        .body
        .columns
        .get(place.0)
        .map_or(FactCells(&[]), |cells| FactCells(cells))
}

/// Whether the table is indexed by `column`: the column it is filled by, or another its catalog item declares among its
/// keys. `rows_of` and `presence` answer for such a column alone.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::reading::service::indexed_by;
/// use ryo_language::domain::fact_tables::types::FactTable;
/// use ryo_relations::domain::relations::collections::relations::comment::item::CommentColumn;
/// use ryo_relations::domain::relations::types::{Column, Count};
///
/// assert!(!indexed_by(&FactTable::default(), Column::Comment(CommentColumn::Id)));
/// ```
#[must_use]
pub fn indexed_by(table: &FactTable, column: Column) -> bool {
    table.body.indexes.iter().any(|index| index.column == column)
}
