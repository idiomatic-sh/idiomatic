//! Each family leaves through the production entry that acquires it; every exit ends with the
//! owner generation retired and `(required, optional, unique) == (0, 0, 0)` asserted here.
//! `RawKeyOrder` needs a `KernelPermit`, so its exits are the program-level `raw_scope` cases.
use crate::domain::operating::errors::OperatingError;
use ryo_derived::domain::request::types::DerivedKind;
use self::exit::{Entry, Exit, entry};

pub(in crate::application::file_dag) mod exit;
mod graph;
mod partition;
mod selection;
mod workspace;

/// Drive one exit of `kind` through its production entry. `RawKeyOrder` is not driven here.
pub(in crate::application::file_dag) fn leave(
    kind: DerivedKind,
    exit: Exit,
) -> Result<DerivedKind, OperatingError> {
    match entry(kind) {
        Entry::Workspace => workspace::workspace(kind, exit),
        Entry::Partition => partition::partition(kind, exit),
        Entry::Selection => selection::selection(kind, exit),
        Entry::Graph => graph::graph(kind, exit),
        Entry::Permit => Err(OperatingError::WrongOperation),
    }
}
