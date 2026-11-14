//! The `project_source` service: the project's relations, read once per phase into the phase's symbols, and handed to each rule
//! that reads one after the files.

use super::types::{PhaseRead, ProjectRelation, ProjectSource, ProjectWritten};
use crate::outbound::facts::errors::FactsError;
use ryo_execution_domain::domain::finish::service::carried_once;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::catalog::service::scope_of;
use ryo_relations::domain::catalog::types::RowScope;
use ryo_relations::domain::relations::service::all;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::rows::types::Table;
use std::sync::Arc;

/// The project's source over what core's project read wrote, serving every relation the catalog declares `scope Project`, in
/// catalog order.
pub(crate) fn opened(written: ProjectWritten<'_>) -> ProjectSource<'_> {
    ProjectSource {
        written,
        serves: all()
            .iter()
            .copied()
            .filter(|relation| scope_of(*relation) == RowScope::Project)
            .collect(),
        read: PhaseRead::Unread,
    }
}

/// Reads each of `wanted` it serves, once for the phase: the project's rows carried into `symbols`, a repeated row kept once,
/// the first in place. A second read reads nothing.
pub(crate) fn read(source: &mut ProjectSource<'_>, symbols: &mut Symbols, wanted: &[Relation]) {
    if matches!(source.read, PhaseRead::Read(_)) {
        return;
    }
    let mut relations: Vec<ProjectRelation> = Vec::new();
    for relation in wanted {
        if !source.serves.contains(relation) || relations.iter().any(|read| read.relation == *relation) {
            continue;
        }
        let rows = source
            .written
            .rows
            .rows(*relation)
            .map(|view| Arc::new(carried_once(view, source.written.symbols, symbols)));
        relations.push(ProjectRelation {
            relation: *relation,
            rows,
        });
    }
    source.read = PhaseRead::Read(relations);
}

/// The rows of `relation`, as the phase read them.
///
/// # Errors
///
/// The project worker's refusal of it, or `NotEnsured` when the phase did not read it.
pub(crate) fn table_of<'s>(source: &'s ProjectSource<'_>, relation: Relation) -> Result<&'s Arc<Table>, FactsError> {
    let PhaseRead::Read(read) = &source.read else {
        return Err(FactsError::NotEnsured { relation });
    };
    match read.iter().find(|each| each.relation == relation) {
        Some(ProjectRelation { rows: Ok(rows), .. }) => Ok(rows),
        Some(ProjectRelation {
            rows: Err(error), ..
        }) => Err(error.clone()),
        None => Err(FactsError::NotEnsured { relation }),
    }
}
