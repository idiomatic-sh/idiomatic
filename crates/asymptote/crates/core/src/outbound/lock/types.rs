//! The types of the `lock` port: plain data, public fields and derives, with no constructor and no inherent impl.

use std::collections::BTreeMap;

use crate::domain::operations::types::{Operation, OperationId};

/// The operation files of a lock, sorted by id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Listing {
    /// The operations whose files the lock holds.
    pub files: Vec<OperationId>,
}

/// The operations of a lock, by id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Operations(pub BTreeMap<OperationId, Operation>);
