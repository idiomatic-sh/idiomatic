//! Each family leaves through the production entry that acquires it; every exit ends with the
//! owner generation retired, and answers the `(required, optional, unique)` bytes still charged then.
use self::exit::{Entry, Exit, entry};
use super::departed::{Departed, Refused};
use ryo_derived::domain::request::types::DerivedKind;

pub mod exit;
mod graph;
mod partition;
mod workspace;

/// Drive one exit of `kind` through its production entry and answer what it observed.
///
/// # Errors
///
/// The production refusal the exit drives the family into, or the first refusal of its setup, with
/// what the route observed.
pub fn leave(kind: DerivedKind, exit: Exit) -> Result<Departed, Refused> {
    match entry(kind) {
        Entry::Workspace => workspace::workspace(kind, exit),
        Entry::Partition => partition::partition(kind, exit),
        Entry::Graph => graph::graph(kind, exit),
    }
}
