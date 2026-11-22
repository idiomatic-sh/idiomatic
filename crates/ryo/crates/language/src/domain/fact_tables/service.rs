//! The services of the `fact_tables` concept: the one request of a file, the one way a language writes its answer, and
//! the one lookup the engine reads it by, whatever a table's layout and whatever wrote it.

use super::keying::service::{EMPTY, emptied, indexes_of, key_written, keyed, place_of};
use super::types::{
    FactCells, FactRequest, FactSet, FactTable, FactWriter, PartialSaid, RelationBits,
};
use crate::domain::facts::types::{FactAnswer, PartialRows};
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::service::{ALL, columns};
use ryo_relations::domain::relations::types::{Count, Relation};

/// The request of a file that asks `relations`.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::service::{asked, request_of};
/// use ryo_relations::domain::relations::types::Relation;
///
/// let request = request_of(&[Relation::Comment, Relation::Comment]);
/// assert_eq!(asked(&request).collect::<Vec<_>>(), [Relation::Comment]);
/// ```
#[must_use]
pub fn request_of(relations: &[Relation]) -> FactRequest {
    let mut bits = vec![RelationBits::default(); ALL.len().div_ceil(64)];
    for relation in relations {
        let place = place_of(*relation).0;
        if let Some(word) = bits.get_mut(place / 64) {
            word.0 |= 1_u64 << (place % 64);
        }
    }
    FactRequest { relations: bits }
}

/// The relations a request asks, in the catalog's order: what a language answers.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::service::{asked, request_of};
///
/// assert_eq!(asked(&request_of(&[])).count(), 0);
/// ```
pub fn asked(request: &FactRequest) -> impl Iterator<Item = Relation> + '_ {
    ALL.iter().enumerate().filter_map(|(place, relation)| {
        request
            .relations
            .get(place / 64)
            .is_some_and(|word| word.0 >> (place % 64) & 1 == 1)
            .then_some(*relation)
    })
}

/// Begins the answer to a file's request in `set`: every relation of the last file is forgotten, the buffers kept, and
/// every relation is unanswered until the language writes or refuses it.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::service::{answer_of, begun, request_of};
/// use ryo_language::domain::fact_tables::types::FactSet;
/// use ryo_language::domain::facts::types::FactAnswer;
/// use ryo_relations::domain::answers::types::AbsenceCause;
/// use ryo_relations::domain::relations::types::Relation;
///
/// let mut set = FactSet::default();
/// begun(&mut set, &request_of(&[Relation::Comment]));
/// assert_eq!(answer_of(&set, Relation::Comment), FactAnswer::Refused(AbsenceCause::NotYetRead));
/// ```
pub fn begun(set: &mut FactSet, request: &FactRequest) {
    let body = &mut set.body;
    body.request.clone_from(request);
    body.tables.resize_with(ALL.len(), FactTable::default);
    body.answers.clear();
    body.answers
        .resize(ALL.len(), FactAnswer::Refused(AbsenceCause::NotYetRead));
    for table in &mut body.tables {
        table.body.rows = Count(0);
        for column in &mut table.body.columns {
            column.clear();
        }
        for index in &mut table.body.indexes {
            emptied(index);
        }
    }
}

/// Begins one relation of the answer: its table emptied, its buffers kept, its indexes the ones its catalog item declares
/// (the column it is filled by, then the other columns it declares among its keys).
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::service::{begun, finished, request_of, table_of, writer};
/// use ryo_language::domain::fact_tables::reading::service::{every_row};
/// use ryo_language::domain::fact_tables::types::FactSet;
/// use ryo_relations::domain::relations::types::Relation;
///
/// let mut set = FactSet::default();
/// begun(&mut set, &request_of(&[Relation::Comment]));
/// finished(writer(&mut set, Relation::Comment));
/// assert_eq!(every_row(table_of(&set, Relation::Comment)).end.0, 0);
/// ```
#[must_use]
pub fn writer(set: &mut FactSet, relation: Relation) -> FactWriter<'_> {
    let place = place_of(relation);
    if set.body.tables.len() <= place.0 {
        set.body.tables.resize_with(ALL.len(), FactTable::default);
    }
    if let Some(table) = set.body.tables.get_mut(place.0) {
        let body = &mut table.body;
        if body.named.is_empty() {
            body.named = columns(relation);
            body.indexes = indexes_of(relation, &body.named);
        }
        body.columns.resize_with(body.named.len(), Vec::new);
        for column in &mut body.columns {
            column.clear();
        }
        for index in &mut body.indexes {
            emptied(index);
        }
        body.rows = Count(0);
    }
    FactWriter {
        set,
        place,
        partial: PartialSaid::Whole,
    }
}

/// Writes one row: its cells in the relation's column order. A row of another width is a fault of the language's filler
/// and is not kept.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::service::{begun, finished, request_of, row, table_of, writer};
/// use ryo_language::domain::fact_tables::reading::service::{every_row};
/// use ryo_language::domain::fact_tables::types::{FactCells, FactSet};
/// use ryo_language::domain::facts::types::CellWord;
/// use ryo_relations::domain::relations::types::Relation;
///
/// let mut set = FactSet::default();
/// begun(&mut set, &request_of(&[Relation::LanguageTarget]));
/// let mut writing = writer(&mut set, Relation::LanguageTarget);
/// row(&mut writing, FactCells(&[CellWord(1), CellWord(2), CellWord(3)]));
/// finished(writing);
/// assert_eq!(every_row(table_of(&set, Relation::LanguageTarget)).end.0, 1);
/// ```
pub fn row(writer: &mut FactWriter<'_>, cells: FactCells<'_>) {
    let Some(table) = writer.set.body.tables.get_mut(writer.place.0) else {
        return;
    };
    let body = &mut table.body;
    debug_assert_eq!(
        cells.0.len(),
        body.columns.len(),
        "a row's cells are its relation's columns"
    );
    if cells.0.len() != body.columns.len() {
        return;
    }
    key_written(&mut body.indexes, cells.0);
    for (column, cell) in body.columns.iter_mut().zip(cells.0) {
        column.push(*cell);
    }
    body.rows = Count(body.rows.0.saturating_add(1));
}

/// Says some rows may be missing: the first thing said is kept, and the relation's answer is `Partial`.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::service::{answer_of, begun, finished, partial, request_of, writer};
/// use ryo_language::domain::fact_tables::types::FactSet;
/// use ryo_language::domain::facts::types::{FactAnswer, FillerSentence, PartialRows};
/// use ryo_relations::domain::relations::types::{Missing, Relation};
///
/// let mut set = FactSet::default();
/// begun(&mut set, &request_of(&[Relation::Comment]));
/// let mut writing = writer(&mut set, Relation::Comment);
/// let said = PartialRows { cause: FillerSentence("an unread construct".into()), missing: Missing::Any };
/// partial(&mut writing, said.clone());
/// finished(writing);
/// assert_eq!(answer_of(&set, Relation::Comment), FactAnswer::Partial(said));
/// ```
pub fn partial(writer: &mut FactWriter<'_>, rows: PartialRows) {
    if writer.partial == PartialSaid::Whole {
        writer.partial = PartialSaid::Partial(rows);
    }
}

/// Ends one relation of the answer: its rows put in the order of the key it is filled by, a key's rows in the order they
/// were written, and each of its indexes built. Its answer is `Partial` when the language said some rows may be missing,
/// else `Answered`.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::service::{answer_of, begun, finished, request_of, writer};
/// use ryo_language::domain::fact_tables::types::FactSet;
/// use ryo_language::domain::facts::types::FactAnswer;
/// use ryo_relations::domain::relations::types::Relation;
///
/// let mut set = FactSet::default();
/// begun(&mut set, &request_of(&[Relation::Comment]));
/// finished(writer(&mut set, Relation::Comment));
/// assert_eq!(answer_of(&set, Relation::Comment), FactAnswer::Answered);
/// ```
pub fn finished(writer: FactWriter<'_>) {
    let FactWriter {
        set,
        place,
        partial,
    } = writer;
    if let Some(table) = set.body.tables.get_mut(place.0) {
        keyed(&mut table.body);
    }
    if let Some(answer) = set.body.answers.get_mut(place.0) {
        *answer = match partial {
            PartialSaid::Whole => FactAnswer::Answered,
            PartialSaid::Partial(rows) => FactAnswer::Partial(rows),
        };
    }
}

/// Answers that the relation is not in the language for this file, for `cause`: its table holds no row.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::service::{answer_of, begun, refused, request_of};
/// use ryo_language::domain::fact_tables::types::FactSet;
/// use ryo_language::domain::facts::types::FactAnswer;
/// use ryo_relations::domain::answers::types::AbsenceCause;
/// use ryo_relations::domain::relations::types::Relation;
///
/// let mut set = FactSet::default();
/// begun(&mut set, &request_of(&[Relation::Comment]));
/// refused(&mut set, Relation::Comment, AbsenceCause::NoSuchConstruct);
/// assert_eq!(answer_of(&set, Relation::Comment), FactAnswer::Refused(AbsenceCause::NoSuchConstruct));
/// ```
pub fn refused(set: &mut FactSet, relation: Relation, cause: AbsenceCause) {
    let place = place_of(relation);
    drop(writer(set, relation));
    if let Some(answer) = set.body.answers.get_mut(place.0) {
        *answer = FactAnswer::Refused(cause);
    }
}

/// What the language answered for a relation of the file: `Refused(NotYetRead)` when it was not asked or not answered.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::service::answer_of;
/// use ryo_language::domain::fact_tables::types::FactSet;
/// use ryo_language::domain::facts::types::FactAnswer;
/// use ryo_relations::domain::answers::types::AbsenceCause;
/// use ryo_relations::domain::relations::types::Relation;
///
/// assert_eq!(answer_of(&FactSet::default(), Relation::Comment), FactAnswer::Refused(AbsenceCause::NotYetRead));
/// ```
#[must_use]
pub fn answer_of(set: &FactSet, relation: Relation) -> FactAnswer {
    match set.body.answers.get(place_of(relation).0) {
        Some(answer) => answer.clone(),
        None => FactAnswer::Refused(AbsenceCause::NotYetRead),
    }
}

/// The table of a relation of the file: an empty one for a relation not answered.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::service::{table_of};
/// use ryo_language::domain::fact_tables::reading::service::{every_row};
/// use ryo_language::domain::fact_tables::types::{FactRowRun, FactSet};
/// use ryo_relations::domain::relations::types::Relation;
///
/// assert_eq!(every_row(table_of(&FactSet::default(), Relation::Comment)), FactRowRun::default());
/// ```
#[must_use]
pub fn table_of(set: &FactSet, relation: Relation) -> &FactTable {
    match set.body.tables.get(place_of(relation).0) {
        Some(table) => table,
        None => &EMPTY,
    }
}
