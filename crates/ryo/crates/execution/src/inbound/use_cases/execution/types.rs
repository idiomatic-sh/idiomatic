//! Public request data and opaque prepared-program handle.
pub use crate::domain::prepared_request::types::ObservationIdentity;
pub use crate::outbound::derived::interface::DerivedPort;
pub use crate::outbound::operators::interface::OperatorsPort;
use std::sync::Arc;
pub struct PreparedRequest {
    pub plans: Vec<ryo_values::domain::plans::types::Plan>,
    pub bindings: Vec<Vec<ryo_values::domain::query_bindings::types::ParameterBinding>>,
    pub rules: Vec<ryo_values::domain::rule_requests::types::RuleSpec>,
    pub digests: Vec<ryo_values::domain::stored_facts::types::Digest>,
    pub hosted: Vec<ryo_values::domain::hosted::types::HostRelation>,
    pub observations: ObservationIdentity,
}
pub struct PreparedProgram {
    pub(super) kernels: Arc<dyn crate::application::file_dag::executor::PrimitiveExecutor + Send>,
    pub(super) derived: Arc<dyn DerivedPort>,
    pub(super) inner: Arc<crate::application::file_dag::program::facade::PreparedProgram>,
}

pub use crate::application::file_dag::frontiers::types::{
    BoundInput, BoundMember, PreparedFrontiers, PublicationSlot,
};

pub mod facts {
    pub use crate::outbound::facts::errors::FactsError;
    pub use crate::outbound::facts::interface::{FactsFinish, FactsPort, FactsWorker};
    pub use crate::outbound::facts::types::{
        BucketSource, ColumnsMut, KeptBucket, ResidentStability, WordsToKeep,
    };
}
pub mod digest {
    pub use crate::outbound::digest::interface::DigestPort;
    pub use crate::outbound::digest::types::{DigestBytes, DigestWords};
}
pub mod matching {
    pub use crate::outbound::matching::errors::MatchingError;
    pub use crate::outbound::matching::interface::MatchingPort;
}

// These are the deliberate data/error signatures of the inbound run protocol;
// no execution owner, permit, source proof constructor or mutable program escapes.
pub use crate::application::file_dag::program::admission::{
    ApplicabilityProof, ApplicabilityReason,
};
pub use crate::application::file_dag::program::facade::{
    CaptureContext, CapturedFrontier, EmitDemand, FrontierCapture, LogicalCounts,
    ObservationDemand, ProjectObservation, ProjectRuleOutcome, RootOutcome, RuleDemand,
    RuleOutcome,
};
pub use crate::domain::coverage::types::{Covered, Partial, Partials, RuleIndex};
pub use crate::domain::kind_sets::types::PreparedVocabulary;
pub use crate::domain::operating::errors::OperatingError;
pub use crate::domain::operating::types::SemanticLookup;
pub use crate::domain::table_model::types::Rows;
pub struct RequestExecution {
    pub(super) program: Arc<crate::application::file_dag::program::facade::PreparedProgram>,
    pub(super) services:
        Arc<crate::application::file_dag::program::facade_runtime::ProgramServices>,
}
pub struct FileInput<'a, 'facts> {
    pub file: ryo_relations::domain::relations::types::FileId,
    pub evaluation: &'a [Partial],
    pub publication: &'a [Partial],
    pub demanded: &'a [bool],
    pub source: &'a [u8],
    /// The immutable provider snapshot is bound once, never replaced per rule.
    pub read: &'a dyn Fn(
        ryo_relations::domain::relations::types::Relation,
    ) -> Result<Rows<'facts>, OperatingError>,
    pub semantic: Option<&'a dyn SemanticLookup>,
    pub kinds: Option<Arc<PreparedVocabulary>>,
}
pub struct FileSession<'a, 'facts> {
    pub(super) inner: crate::application::file_dag::program::facade_runtime::FileRun<'a>,
    pub(super) read: &'a dyn Fn(
        ryo_relations::domain::relations::types::Relation,
    ) -> Result<Rows<'facts>, OperatingError>,
}
pub struct ProjectScope {
    pub(super) required: crate::application::file_dag::program::admission::RequiredPartitions,
    pub(super) publication: crate::application::file_dag::program::facade_runtime::PublicationScope,
}
pub struct ProjectInput<'a> {
    pub scope: &'a ProjectScope,
    pub evaluation: &'a Partials,
    pub publication: &'a Partials,
    pub ordinary: PreparedFrontiers,
    pub bound: &'a [BoundInput],
    pub arenas: &'a [ryo_values::domain::stored_facts::types::ArenaBytes],
}
pub struct ProjectSession<'a> {
    pub(super) inner: crate::application::file_dag::program::facade_runtime::ProjectRun<'a>,
}

pub struct FileFinished {
    pub readiness: Option<Arc<ryo_values::domain::records::types::FrontierReadiness>>,
}
pub struct FrontierRequest<'a> {
    pub scope: &'a ProjectScope,
    pub rule: RuleIndex,
    pub first: ryo_values::domain::stored_facts::types::Bucket,
    pub facts: &'a dyn facts::FactsFinish,
    pub project: &'a dyn facts::FactsWorker,
    pub project_symbols: &'a ryo_language::domain::symbols::types::Symbols,
}
pub use crate::application::file_dag::frontiers::service::PreparationError;
