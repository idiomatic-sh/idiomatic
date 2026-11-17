//! The open file's one read of each fact relation, made at its first ask and answered to every later one.
use crate::application::file_dag::fact_lookup::service::FactLookup;
use crate::application::file_dag::file_pass::service::reads::types::{FactRead, FileReads};
use crate::outbound::facts::errors::FactsError;
use ryo_relations::domain::relations::types::{ColumnsView, Relation};
use std::cell::RefCell;

/// `relation`'s rows in the open file, or the lookup's refusal: the file's one read of it, made at its first ask and answered
/// to every later one, so a relation's read is one value for the whole file whichever reader asks first.
pub(crate) fn read<'p>(
    reads: &RefCell<FileReads<'p>>,
    lookup: &'p FactLookup<'_, '_>,
    relation: Relation,
) -> Result<ColumnsView<'p>, FactsError> {
    if let Some(known) = reads
        .borrow()
        .0
        .iter()
        .find(|read| read.relation == relation)
    {
        return known.read.clone();
    }
    let answered = lookup.columns(relation);
    reads.borrow_mut().0.push(FactRead {
        relation,
        read: answered.clone(),
    });
    answered
}
