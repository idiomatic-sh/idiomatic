//! The types of the `renames` concept: plain data, public fields and derives, with no constructor and no inherent
//! impl.

use std::collections::BTreeMap;

use crate::domain::operations::types::{OperationId, Record, Removal, Revision};
use crate::domain::violations::types::{Id, RepoPath};

/// One file git calls renamed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rename {
    /// Its old path, relative to the product.
    pub from: RepoPath,
    /// Its new one.
    pub to: RepoPath,
}

/// One first-parent commit and the renames it made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenameEvent {
    /// The commit.
    pub revision: Revision,
    /// Its renames.
    pub renames: Vec<Rename>,
}

/// A commit's first parent, when it has one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Parent {
    /// This commit.
    Commit(Revision),
    /// None: the root commit.
    Root,
}

/// The first-parent commit that added one file of the lock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Introduction {
    /// The file, relative to the product.
    pub path: RepoPath,
    /// The commit that added it.
    pub revision: Revision,
    /// That commit's first parent.
    pub parent: Parent,
}

/// Where a history of renames starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryStart {
    /// At the first commit.
    Beginning,
    /// After this commit.
    After(Revision),
}

/// The commit a working tree stands on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Head {
    /// This commit.
    Commit(Revision),
    /// None: no repository, or one with no commit.
    Detached,
}

/// A record carried through a rename.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Migration {
    /// The record as the lock holds it.
    pub original: Record,
    /// The record keyed at its new path.
    pub projected: Record,
}

/// Which committed renames apply to a record, by where its history starts.
#[derive(Clone, Copy, Debug)]
pub enum Basis<'a> {
    /// From the earliest commit that added an operation accepting it, found by position in the history.
    Operations {
        /// Each record's accepting operations.
        origins: &'a BTreeMap<Id, Vec<OperationId>>,
        /// The commit that added each operation.
        introduced: &'a BTreeMap<OperationId, Revision>,
    },
    /// Every committed rename: the history already starts after a merge base.
    Branch,
    /// None is known to apply; only the working tree's do.
    Unknown,
}

/// A run's rename projections, split by whether it can carry them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Carried {
    /// The projections whose identity the run reports at the new path.
    pub carried: Vec<Migration>,
    /// The ones it does not: carrying one would accept a violation nothing reports.
    pub unmatched: Vec<Migration>,
}

/// Records carried through renames.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Projection {
    /// Every record, each at its path now.
    pub records: Vec<Record>,
    /// The ones that moved.
    pub migrations: Vec<Migration>,
}

/// The records an operation adds and removes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Changes {
    /// Added.
    pub added: Vec<Record>,
    /// Removed.
    pub removed: Vec<Removal>,
}
