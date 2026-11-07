//! The publication port, implemented with the execution use case.
use crate::outbound::publication::interface::PublicationPort;
use ryo_execution::application::publication::{service::publication, types::PublicationInput};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{NamePlace, WorkerNumber};
use ryo_values::domain::{
    records::types::ObservationReason, rows::types::Table, stored_facts::types::ArenaBytes,
};

/// Stateless translation to execution-owned producer normalization.
#[derive(Debug)]
pub(crate) struct PublicationExecutionAdapter;

impl PublicationExecutionAdapter {
    pub(crate) const fn new() -> Self {
        Self
    }
}
impl PublicationPort for PublicationExecutionAdapter {
    fn publication(
        &self,
        table: &mut Table,
        symbols: &mut Symbols,
        inherited: &[ArenaBytes],
        worker: WorkerNumber,
        arena: &mut ArenaBytes,
        places: &mut Vec<NamePlace>,
    ) -> Result<(), ObservationReason> {
        publication(PublicationInput {
            table,
            symbols,
            inherited,
            worker,
            arena,
            places,
        })
        .map_err(|error| ObservationReason(error.to_string().into()))
    }
}
