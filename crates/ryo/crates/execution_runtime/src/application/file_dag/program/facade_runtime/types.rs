//! Data of one execution's shared services, its publication scope and its file roots.
use crate::application::file_dag::executor::PrimitiveExecutor;
use crate::application::file_dag::program::admission::AdmittedInput;
use crate::application::file_dag::program::admission::ApplicabilityProof;
use crate::application::file_dag::program::admission::AvailableInput;
use crate::application::file_dag::program::admission::RequiredPartitions;
use crate::application::file_dag::program::request_identity::RequestIdentity;
use crate::application::file_dag::program::runtime::service::ProgramExecution;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::relations::RelationOwner;
use ryo_execution_store::application::file_dag::relations::interface::RelationStore as _;
use std::{rc::Rc, sync::Arc};

/// Whether the frontier a file root publishes has been committed by its owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FrontierCommit {
    Pending,
    Committed,
}
/// Whether the native walk of a file has bound its roots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WalkPreparation {
    Pending,
    Prepared,
}
/// Whether the sources a named root reads are free of blocking coverage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NamedReadiness {
    Ready,
    Blocked,
}
/// A named observation output of one rule, prepared for admission.
#[derive(Clone, Copy, Debug)]
pub(super) struct PreparedNamed {
    pub(super) rule: ryo_execution_domain::domain::coverage::types::RuleIndex,
    pub(super) output: ryo_values::domain::records::types::ObservationIndex,
    pub(super) readiness: NamedReadiness,
}
/// The delivery, observation and capture a file declared for one rule.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DeclaredIntent {
    pub(crate) delivery: ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery,
    pub(crate) demand: ryo_execution_domain::domain::rule_intent::types::ObservationDemand,
    pub(crate) capture_intent: ryo_execution_domain::domain::rule_intent::types::CaptureIntent,
}
/// The slot of a bound observation among the observation regions of a program.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct ObservationKey(pub(super) usize);
/// Where an ordinary project frontier sits among those a run was given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct OrdinaryPlace(pub(super) usize);
/// Whether a run already holds the input a source names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Provision {
    Provided,
    Absent,
}
/// A project source admitted once for the run, with the source it answers.
pub(super) struct CachedInput {
    pub(super) source: ryo_execution_domain::domain::executable_program::types::InputSource,
    pub(super) input: AvailableInput,
}

/// What one presence test or formula answered on the current file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Truth {
    Unknown,
    Holds,
    Fails,
}
/// Whether a file decides presence: off on any file with a partial read, where every root runs in full.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PresenceGate {
    On,
    Off,
}
/// A run rule's prepared root, which the presence gate may find absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RootCandidate {
    pub(super) rule: ryo_execution_domain::domain::coverage::types::RuleIndex,
    pub(super) region: ryo_execution_domain::domain::executable_program::types::RegionId,
}
/// Whether a file instantiated a region: a slot, only a dormant reader count, or nothing at all.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Instantiated {
    Never,
    Slot,
    Dormant,
}
/// Whether a region's inputs are released, or still owed by it.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InputRelease {
    Released,
    Owed,
}
/// What a file left of one region, observed only by tests.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RegionProbe {
    pub(crate) instantiated: Instantiated,
    pub(crate) inputs: Option<InputRelease>,
}
/// What a file binds when its first entry point reads it: its selected readers and the roots presence may find absent.
pub(super) struct PendingActivation {
    pub(super) readers: Vec<ryo_execution_domain::domain::executable_program::types::RegionId>,
    pub(super) candidates: Vec<RootCandidate>,
    pub(super) gate: PresenceGate,
}
pub(super) struct FileRoot {
    pub(super) readiness: Option<ryo_values::domain::records::types::ProducerReadiness>,
    pub(super) commit: FrontierCommit,
    pub(super) publication: Option<AdmittedInput>,
    pub(super) skipped: Option<ApplicabilityProof>,
}
