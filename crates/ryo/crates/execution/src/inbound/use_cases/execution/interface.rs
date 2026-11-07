//! What the `execution` use case promises its callers, in the core's own vocabulary.
use crate::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use crate::application::file_dag::program::request_execution::types::RequestExecution;
use ryo_values::domain::files::types::CorpusInput;

// Canonical inbound signature vocabulary from private pure concepts.
pub use crate::domain::coverage::types::Covered;
pub use crate::domain::coverage::types::FileObservationReadiness;
pub use crate::domain::coverage::types::FilePartial;
pub use crate::domain::coverage::types::InapplicableFiles;
pub use crate::domain::coverage::types::Lacking;
pub use crate::domain::coverage::types::LackingKind;
pub use crate::domain::coverage::types::Partial;
pub use crate::domain::coverage::types::Partials;
pub use crate::domain::coverage::types::RuleIndex;
pub use crate::domain::file_program::errors::FileProgramError;
pub use crate::domain::file_program::types::ObservedFrontierRoot;
pub use crate::domain::frontier_capture::types::CapturedFrontier;
pub use crate::domain::frontier_capture::types::FrontierCapture;
pub use crate::domain::kind_sets::types::vocabulary::value::PreparedVocabulary;
pub use crate::domain::operator::types::Counting;
pub use crate::domain::rule_intent::types::FrontierPurpose;
pub use crate::domain::finish::types::Arena;
pub use crate::domain::finish::types::PublicationReason;
pub use crate::domain::operating::errors::OperatingError;
pub use crate::domain::operating::types::SemanticLookup;
pub use crate::domain::operator::types::LogicalCounts;
pub use crate::domain::operator::types::RowsFed;
pub use crate::domain::readiness::types::InvocationStateCount;
pub use crate::domain::readiness::types::InvocationStateCounts;
pub use crate::domain::readiness::types::ReadinessCounts;
pub use crate::domain::readiness::types::ReadinessEvents;
pub use crate::domain::row_emission::types::RowEmission;
pub use crate::domain::rule_intent::types::ApplicabilityReason;
pub use crate::domain::rule_intent::types::ApplicabilityRefusal;
pub use crate::domain::rule_intent::types::CaptureIntent;
pub use crate::domain::rule_intent::types::DiagnosticDelivery;
pub use crate::domain::rule_intent::types::EmitDemand;
pub use crate::domain::rule_intent::types::FileRuleIntent;
pub use crate::domain::rule_intent::types::ObservationDemand;
pub use crate::domain::rule_intent::types::RuleDemand;
pub use crate::domain::shared_walk::types::WalkClassWork;
pub use crate::domain::shared_walk::types::WalkCounts;
pub use crate::domain::shared_walk::types::WalkDecodedRows;
pub use crate::domain::shared_walk::types::WalkPassCount;
pub use crate::domain::shared_walk::types::WalkPredicateRows;
pub use crate::domain::shared_walk::types::WalkPredicateTests;
pub use crate::domain::shared_walk::types::WalkSourceRows;
pub use crate::domain::spelling::errors::SpellingError;
pub use crate::domain::trust::types::RelationRead;

/// What `execution` offers its callers.
pub trait ExecutionUseCase {
    /// Begins a request execution over a prepared program and the corpus it runs on.
    fn execution(&self, program: &PreparedExecutionProgram, corpus: &[CorpusInput]) -> RequestExecution;
}
