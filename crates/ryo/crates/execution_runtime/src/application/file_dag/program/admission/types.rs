//! The types of the `admission` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use super::service::{AdmittedInput, FileGeneration};
use ryo_execution_domain::domain::{
    coverage::types::RuleIndex,
    executable_program::types::{InputSource, RegionId},
    operating::errors::OperatingError,
    rule_intent::types::ApplicabilityReason,
};
use ryo_execution_store::application::file_dag::relations::interface::RelationsPort as _;
use ryo_execution_store::application::file_dag::{
    derived::population::service::PopulationId,
    relations::{handle::service::Publication, service::RelationOwner},
};
use ryo_relations::domain::relations::types::FileId;
use std::{rc::Rc, sync::Arc};

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

#[derive(Clone)]
pub(crate) enum Partition {
    RequestInvariant,
    File(FileGeneration),
    Project(RequiredPartitions),
}

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

#[derive(Clone)]
pub(super) enum AvailableBody {
    Published(AdmittedInput),
    Source {
        owner: Rc<RelationOwner>,
        publication: Publication,
        partition: Partition,
        admission: Result<AdmittedInput, OperatingError>,
    },
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
