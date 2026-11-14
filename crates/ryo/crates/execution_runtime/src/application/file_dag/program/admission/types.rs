//! The types of the `admission` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_execution_domain::domain::interpreted_program::types::{InputSource, RegionId};
use ryo_relations::domain::relations::types::FileId;
use std::sync::Arc;

/// Whether two admitted identities are the same.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Agreement {
    Same,
    Different,
}

/// The project identity is the request and its files, coverage and skips. `inapplicable` and
/// `refused` are one rule's own evidence beside that identity: equality never reads them, so every
/// identity check (open, supplies, materialize, the frontier cache key) agrees across rules.
#[derive(Clone)]
pub(crate) struct RequiredPartitions {
    pub(crate) request: Arc<super::super::request_identity::service::RequestIdentity>,
    pub(crate) files: Box<[FileId]>,
    pub(crate) covered: Box<[ryo_execution_domain::domain::coverage::types::Covered]>,
    pub(crate) skipped: Box<[ryo_values::domain::runs::types::Skipped]>,
    pub(crate) inapplicable: Box<[FileId]>,
    pub(crate) refused: Box<[FileId]>,
}

impl PartialEq for RequiredPartitions {
    fn eq(&self, other: &Self) -> bool {
        self.request == other.request
            && self.files == other.files
            && self.covered == other.covered
            && self.skipped == other.skipped
    }
}

impl Eq for RequiredPartitions {}

/// Whether an admitted input has no rows in a given representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Emptiness {
    Empty,
    Populated,
}

/// Root provenance remains attached even when no rows can be supplied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProducerRoot {
    Input(InputSource),
    Region(RegionId),
}

/// Whether a partition covers the partition a consumer requires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PartitionSupply {
    Supplies,
    Lacks,
}

/// Whether an admitted input is complete enough to feed a named observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ObservationReadiness {
    Ready,
    Waiting,
}

/// The observation slot an alias is issued for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AliasSlot(pub usize);

/// The member of a named observation an alias is issued for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AliasMember(pub usize);
