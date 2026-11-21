//! The services of the `filled` concept: a relation asked of the language whole or for one key, each once per file.

use super::errors::FilledError;
use super::types::{AskedRows, FilledRelations, RelationAsk};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::Count;

/// Every row of the relation: asked whole of the language the first time, its rows written after those the keys asked
/// before appended; the rows of the whole fill after that.
///
/// # Errors
///
/// The language refused the relation or wrote it in part.
pub(crate) fn whole(
    relations: &mut FilledRelations,
    asking: RelationAsk<'_>,
) -> Result<AskedRows, FilledError> {
    let _unbuilt = (relations, asking.facts, asking.words, asking.stamp, asking.number);
    Err(FilledError::Refused {
        relation: asking.relation,
        cause: AbsenceCause::NotYetRead,
    })
}

/// The rows the language wrote for one key, asked of it the first time the key is asked while the relation is not whole.
/// A language that answers every row makes the relation whole: the answer says so, and the rows are then every row.
///
/// # Errors
///
/// The language refused the relation or wrote it in part.
pub(crate) fn keyed(
    relations: &mut FilledRelations,
    asking: RelationAsk<'_>,
    column: Count,
    cell: CellWord,
) -> Result<AskedRows, FilledError> {
    let _unbuilt = (relations, asking.facts, asking.words, asking.stamp, asking.number, column, cell);
    Err(FilledError::Refused {
        relation: asking.relation,
        cause: AbsenceCause::NotYetRead,
    })
}
