//! The publication port, implemented with the execution use case.
use crate::outbound::publication::interface::PublicationPort;
use ryo_execution::inbound::use_cases::publication::{
    interface::PublicationUseCase, types::PublicationInput,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{NamePlace, WorkerNumber};
use ryo_values::domain::{
    observations::types::ObservationReason, rows::types::Table, stored_facts::types::ArenaBytes,
};
use std::sync::Arc;

/// Stateless translation to execution-owned producer normalization.
#[derive(Debug)]
pub(crate) struct PublicationExecutionAdapter(Arc<dyn PublicationUseCase>);

impl PublicationExecutionAdapter {
    pub(crate) fn new(publication: Arc<dyn PublicationUseCase>) -> Self {
        Self(publication)
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
        self.0
            .publication(PublicationInput {
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
