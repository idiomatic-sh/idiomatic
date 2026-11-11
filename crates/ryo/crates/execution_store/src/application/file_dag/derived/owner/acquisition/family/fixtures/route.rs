//! Each family leaves through the production entry that acquires it; every exit ends with the
//! owner generation retired and `(required, optional, unique) == (0, 0, 0)` asserted here.
use self::exit::{Entry, Exit, entry};
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::operating::errors::OperatingError;

pub mod exit;
mod graph;
mod partition;
mod workspace;

/// Drive one exit of `kind` through its production entry.
///
/// # Errors
///
/// The production refusal the exit drives the family into, or the first refusal of its setup.
pub fn leave(kind: DerivedKind, exit: Exit) -> Result<DerivedKind, OperatingError> {
    match entry(kind) {
        Entry::Workspace => workspace::workspace(kind, exit),
        Entry::Partition => partition::partition(kind, exit),
        Entry::Graph => graph::graph(kind, exit),
    }
}
