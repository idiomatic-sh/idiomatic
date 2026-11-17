//! Borrowed publication data; callers own output storage and rollback.
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{NamePlace, WorkerNumber};
use ryo_values::domain::{rows::types::Table, stored_facts::types::ArenaBytes};

/// What one publication reads and writes: the table, the symbols its names come from, and the storage its portable names go to.
#[derive(Debug)]
pub struct PublicationInput<'a> {
    /// The table to publish; its producer-local names are rewritten as portable ones.
    pub table: &'a mut Table,
    /// The symbols the table's names are read from.
    pub symbols: &'a mut Symbols,
    /// The arenas inherited from earlier publications.
    pub inherited: &'a [ArenaBytes],
    /// The worker whose arena the publication writes to.
    pub worker: WorkerNumber,
    /// The arena the publication writes the portable names to.
    pub arena: &'a mut ArenaBytes,
    /// The places of the names the publication wrote.
    pub places: &'a mut Vec<NamePlace>,
}
