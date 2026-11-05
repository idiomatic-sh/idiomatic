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
/// The implementation layers are unavailable to callers.
///
/// ```compile_fail,E0603
/// use ryo_execution::application::file_dag::execution::CommonOperatorEngine;
/// ```
/// ```compile_fail,E0603
/// use ryo_execution::domain::executable_program::service::compile;
/// ```
/// ```compile_fail,E0603
/// use ryo_execution::application::file_dag::relations::RelationOwner;
/// ```
/// Public prepared handles can be accepted and inspected through the inbound contract.
/// ```
/// use ryo_execution::inbound::use_cases::execution::{
///     interface::PreparedExecution, types::PreparedProgram,
/// };
/// fn count(program: &PreparedProgram) -> usize { program.rule_count() }
/// ```
/// They cannot be minted by a caller.
/// ```compile_fail,E0451
/// use ryo_execution::inbound::use_cases::execution::types::PreparedProgram;
/// let _ = PreparedProgram { inner: panic!(), kernels: panic!(), derived: panic!() };
/// ```
/// ```compile_fail,E0451
/// use ryo_execution::inbound::use_cases::execution::types::FileSession;
/// let _ = FileSession {
///     inner: panic!(), program: panic!(), file: panic!(),
///     elements: panic!(), read: panic!(),
/// };
/// ```
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
    CapturedFrontier, EmitDemand, FrontierCapture, LogicalCounts, ObservationDemand,
    ProjectObservation, ProjectRuleOutcome, RootOutcome, RuleDemand, RuleOutcome,
};
pub use crate::domain::coverage::types::{Covered, Partial, Partials, RuleIndex};
pub use crate::domain::kind_sets::types::PreparedVocabulary;
pub use crate::domain::operating::errors::OperatingError;
pub use crate::domain::operating::types::SemanticLookup;
/// Physical provider data; execution-owned projections and publications cannot be supplied here.
#[derive(Clone, Debug)]
pub enum SourceRows<'a> {
    Stored(ryo_relations::domain::relations::types::ColumnsView<'a>),
    Computed(&'a ryo_values::domain::rows::types::Table),
}
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
    pub elements: &'a ryo_language::domain::elements::types::Elements,
    /// The immutable provider snapshot is bound once, never replaced per rule.
    pub read: &'a dyn Fn(
        ryo_relations::domain::relations::types::Relation,
    ) -> Result<SourceRows<'facts>, OperatingError>,
    pub semantic: Option<&'a dyn SemanticLookup>,
    pub kinds: Option<Arc<PreparedVocabulary>>,
}
pub struct FileSession<'a, 'facts> {
    pub(super) inner: crate::application::file_dag::program::facade_runtime::FileRun<'a>,
    pub(super) program: Arc<crate::application::file_dag::program::facade::PreparedProgram>,
    pub(super) file: ryo_relations::domain::relations::types::FileId,
    pub(super) elements: &'a ryo_language::domain::elements::types::Elements,
    pub(super) read: &'a dyn Fn(
        ryo_relations::domain::relations::types::Relation,
    ) -> Result<SourceRows<'facts>, OperatingError>,
}
pub struct ProjectScope {
    pub(super) required: crate::application::file_dag::program::admission::RequiredPartitions,
    pub(super) publication:
        std::rc::Rc<crate::application::file_dag::program::facade_runtime::PublicationScope>,
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
pub enum FrontierDemand<'a> {
    Diagnostic,
    Observations { ready: &'a [bool] },
}
pub struct FrontierRequest<'a> {
    pub demand: FrontierDemand<'a>,
    pub scope: &'a ProjectScope,
    pub rule: RuleIndex,
    pub first: ryo_values::domain::stored_facts::types::Bucket,
    pub facts: &'a dyn facts::FactsFinish,
    pub project: &'a dyn facts::FactsWorker,
    pub project_symbols: &'a ryo_language::domain::symbols::types::Symbols,
}
pub use crate::application::file_dag::frontiers::service::PreparationError;

pub use crate::domain::file_program::types::ObservedFrontierRoot;
pub use crate::domain::trust::types::RelationRead;

pub mod query_language {
    pub use crate::outbound::query_language::{
        errors::QueryLanguageError, interface::QueryLanguagePort,
    };
}
pub use crate::application::rule_compilation::{
    errors::RuleCompilationError, types::CompiledPlans,
};
/// Request preparation retains its graph privately; callers may inspect source plans only.
pub struct CompiledQueries {
    pub(super) inner: crate::domain::prepared_request::types::Compiled,
}

pub struct PreparedRuleIdentities {
    pub rules: Vec<ryo_values::domain::stores::types::StoredRule>,
    pub observations: ObservationIdentity,
}

/// One project publication generation shared by every ready batch.
pub struct ObservationStage<'a> {
    pub(super) request: &'a RequestExecution,
    pub(super) publication:
        std::rc::Rc<crate::application::file_dag::program::facade_runtime::PublicationScope>,
    pub(super) publications:
        crate::application::file_dag::observations::cross::CrossPublications<'a>,
}
pub use crate::application::file_dag::program::facade::ObservedPublication;

/// Output storage only; source file and rule digest come from the bound session.
pub struct CaptureContext<'a> {
    pub worker: ryo_relations::domain::relations::types::WorkerNumber,
    pub arena: &'a mut Vec<u8>,
    pub places: &'a mut Vec<ryo_relations::domain::relations::types::NamePlace>,
    pub words: &'a mut ryo_relations::domain::relations::types::Words,
}
pub use crate::domain::coverage::types::FileObservationReadiness;
pub use ryo_values::domain::records::types::CrossPublicationTransfer;

pub use crate::domain::operator::types::RowsFed;
pub use ryo_values::domain::records::types::PackedObservedRoot;

pub use crate::domain::operator::types::TreeSteps;
pub use crate::domain::lowering::types::Counting;
