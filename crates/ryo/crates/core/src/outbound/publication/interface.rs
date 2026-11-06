//! Publication capability injected into cross-file work.
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{NamePlace, WorkerNumber};
use ryo_values::domain::{
    rows::types::Table,
    stored_facts::types::ArenaBytes,
    records::types::ObservationReason,
};

pub trait PublicationPort: Send + Sync {
    fn publication(
        &self,
        table: &mut Table,
        symbols: &mut Symbols,
        inherited: &[ArenaBytes],
        worker: WorkerNumber,
        arena: &mut ArenaBytes,
        places: &mut Vec<NamePlace>,
    ) -> Result<(), ObservationReason>;
}
