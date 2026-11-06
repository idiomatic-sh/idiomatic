//! Controllable publication port for core orchestration tests.
use super::super::interface::PublicationPort;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CROSS_FILE, NamePlace, WorkerNumber};
use ryo_values::domain::{
    rows::types::Table,
    stored_facts::types::ArenaBytes,
    records::types::ObservationReason,
};

#[derive(Default)]
pub struct PublicationFake;

impl PublicationPort for PublicationFake {
    fn publication(
        &self,
        _table: &mut Table,
        _symbols: &mut Symbols,
        _inherited: &[ArenaBytes],
        worker: WorkerNumber,
        _arena: &mut ArenaBytes,
        _places: &mut Vec<NamePlace>,
    ) -> Result<(), ObservationReason> {
        if worker == CROSS_FILE {
            Err(ObservationReason(stringify!(reserved_worker).into()))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
