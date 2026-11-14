//! The types of the `project_source` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::interface::FactsWorker;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::rows::types::Table;
use std::sync::Arc;

/// What core's project read wrote: the project's worker and the symbols its rows name.
#[derive(Clone, Copy)]
pub(crate) struct ProjectWritten<'a> {
    /// The worker holding the project's relations.
    pub(crate) rows: &'a dyn FactsWorker,
    /// The symbols they name.
    pub(crate) symbols: &'a Symbols,
}

impl std::fmt::Debug for ProjectWritten<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// The source of every relation the catalog declares `scope Project`, read once per phase: each relation's rows carried into
/// the phase's symbols, a repeated row kept once, the first in place. Its rows name texts made across the files, which only a
/// table holds, so it hands them over as a table rather than lending them as columns.
#[derive(Debug)]
pub(crate) struct ProjectSource<'a> {
    /// What core's project read wrote.
    pub(crate) written: ProjectWritten<'a>,
    /// The relations it serves, in catalog order.
    pub(crate) serves: Vec<Relation>,
    /// What its one read made, once made.
    pub(crate) read: PhaseRead,
}

/// Whether the phase's project relations were read, and what the read made.
#[derive(Debug, Default)]
pub(crate) enum PhaseRead {
    /// Not yet.
    #[default]
    Unread,
    /// Read: each relation asked for, with its columns or why the project's worker refused it.
    Read(Vec<ProjectRelation>),
}

/// One project relation as the phase holds it.
#[derive(Debug)]
pub(crate) struct ProjectRelation {
    /// The relation.
    pub(crate) relation: Relation,
    /// Its rows in the phase's symbols, or the worker's refusal.
    pub(crate) rows: Result<Arc<Table>, FactsError>,
}
