//! Borrowed publication data; callers own output storage and rollback.
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{NamePlace, WorkerNumber};
use ryo_values::domain::{rows::types::Table, stored_facts::types::ArenaBytes};

pub struct PublicationInput<'a> {
    pub table: &'a mut Table,
    pub symbols: &'a mut Symbols,
    pub inherited: &'a [ArenaBytes],
    pub worker: WorkerNumber,
    pub arena: &'a mut ArenaBytes,
    pub places: &'a mut Vec<NamePlace>,
}
