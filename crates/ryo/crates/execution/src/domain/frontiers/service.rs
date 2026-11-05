//! The `frontiers` service: free functions named by verb over the inputs of the cross-file steps of a rule, with no IO and no ports.

use crate::domain::finish::service::carry;
use crate::domain::table_dedup::service::distinct;
use crate::domain::table_model::service::materialize;
use crate::domain::table_model::types::{Frontier, Rows};
use crate::domain::table_words::service::decode_combined;
use hashbrown::HashTable;
use ryo_language::domain::symbols::service::places_mut;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::provenance_of;
use ryo_relations::domain::relations::types::Provenance;
use ryo_relations::domain::relations::types::{ColumnsView, Relation};
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::stored_facts::types::KeptWords;

/// The relation the project wrote that a rule's input is, when it is one: what the project wrote waits in its own handle, not in
/// the buckets of the files.
#[must_use]
pub(crate) fn project_relation(input: Input) -> Option<Relation> {
    match input {
        Input::Relation(relation) if provenance_of(relation) == Provenance::Manifests => {
            Some(relation)
        }
        Input::Relation(_) | Input::Step(_) | Input::BoundObservation(_) => None,
    }
}

/// The rows of a relation the project wrote, as the input of a rule's cross-file steps, their names made again among the names of the
/// finish. Two linked languages that read the same manifest write the same fact twice: a row is one fact by every one of its columns,
/// so a row repeated is kept once, the first in place.
#[must_use]
pub(crate) fn project_input(
    view: ColumnsView<'_>,
    input: Input,
    project: &Symbols,
    cross: &mut Symbols,
) -> Frontier {
    let mut table = Table::default();
    materialize(&Rows::Stored(view), &mut table, &mut Vec::new());
    carry(&mut table, project, cross);
    distinct(&mut table, &mut HashTable::new());
    Frontier { input, table }
}

/// The rows the files handed to an input of a rule, from the words the workers kept, their names made again among the names of the
/// finish.
#[must_use]
pub(crate) fn decode_input(words: &KeptWords, cross: &mut Symbols) -> Table {
    decode_combined(&words.0, places_mut(cross))
}

use ryo_values::domain::rows::types::Table;
