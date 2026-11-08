//! What the `execution` use case promises its callers, in the core's own vocabulary.
use ryo_values::domain::files::types::CorpusInput;

// The application handles and protocol traits the contract is made of.
pub use crate::application::file_dag::frontiers::errors::PreparationError;
pub use crate::application::file_dag::frontiers::types::BoundInput;
pub use crate::application::file_dag::frontiers::types::InputReadiness;
pub use crate::application::file_dag::frontiers::types::PreparedFrontiers;
pub use crate::application::file_dag::frontiers::types::PublicationSlot;
pub use crate::application::file_dag::observations::types::AdmittedOutput;
pub use crate::application::file_dag::observations::types::RuleSelection;
pub use crate::application::file_dag::program::file_session::types::CaptureContext;
pub use crate::application::file_dag::program::file_session::types::EmittedRoot;
pub use crate::application::file_dag::program::file_session::types::FileEnd;
pub use crate::application::file_dag::program::file_session::types::FileRunInput;
pub use crate::application::file_dag::program::file_session::types::FileStop;
pub use crate::application::file_dag::program::file_session::types::FinishedReadiness;
pub use crate::application::file_dag::program::file_session::types::KindSupport;
pub use crate::application::file_dag::program::file_session::types::RanRule;
pub use crate::application::file_dag::program::file_session::types::RuleAfter;
pub use crate::application::file_dag::program::file_session::types::SemanticSupport;
pub use crate::application::file_dag::program::file_session::types::SourceLanguage;
pub use crate::application::file_dag::program::observation_stage::interface::ObservationExecution;
pub use crate::application::file_dag::program::observation_stage::interface::ObservationPublication;
pub use crate::application::file_dag::program::observed_publication::types::ObservedPublication;
pub use crate::application::file_dag::program::preparation::types::Demand;
pub use crate::application::file_dag::program::preparation::types::NamedLayout;
pub use crate::application::file_dag::program::preparation::types::ObservationContract;
pub use crate::application::file_dag::program::preparation::types::ReplayClosure;
pub use crate::application::file_dag::program::preparation::types::SemanticNeed;
pub use crate::application::file_dag::program::prepared_execution::interface::PreparedExecution;
pub use crate::application::file_dag::program::prepared_execution::interface::PreparedReporting;
pub use crate::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
pub use crate::application::file_dag::program::project_phase::interface::ProjectRootExecution;
pub use crate::application::file_dag::program::project_phase::interface::ProjectStage;
pub use crate::application::file_dag::program::project_phase::types::FrontierDemand;
pub use crate::application::file_dag::program::project_phase::types::FrontierRequest;
pub use crate::application::file_dag::program::project_phase::types::PhaseScope;
pub use crate::application::file_dag::program::project_phase::types::ProjectInput;
pub use crate::application::file_dag::program::project_phase::types::ProjectPhase;
pub use crate::application::file_dag::program::project_phase::types::ProjectTask;
pub use crate::application::file_dag::program::request_execution::interface::ExecutionRequest;
pub use crate::application::file_dag::program::request_execution::types::RequestExecution;
pub use crate::application::file_dag::program::rule_outcome::types::ProjectBatch;
pub use crate::application::file_dag::program::rule_outcome::types::ProjectObservation;
pub use crate::application::file_dag::program::rule_outcome::types::ProjectOutputOutcome;
pub use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
pub use crate::application::file_dag::program::rule_outcome::types::RuleOutcome;
// Canonical inbound signature vocabulary from private pure concepts.
pub use ryo_execution_domain::domain::coverage::types::Covered;
pub use ryo_execution_domain::domain::coverage::types::FileObservationReadiness;
pub use ryo_execution_domain::domain::coverage::types::FilePartial;
pub use ryo_execution_domain::domain::coverage::types::InapplicableFiles;
pub use ryo_execution_domain::domain::coverage::types::Lacking;
pub use ryo_execution_domain::domain::coverage::types::LackingKind;
pub use ryo_execution_domain::domain::coverage::types::Partial;
pub use ryo_execution_domain::domain::coverage::types::Partials;
pub use ryo_execution_domain::domain::coverage::types::RuleIndex;
pub use ryo_execution_domain::domain::rule_intent::errors::RuleIntentError;
pub use ryo_execution_domain::domain::file_program::types::ObservedFrontierRoot;
pub use ryo_execution_domain::domain::frontier_capture::types::CapturedFrontier;
pub use ryo_execution_domain::domain::frontier_capture::types::FrontierCapture;
pub use ryo_execution_domain::domain::kind_sets::types::vocabulary::value::PreparedVocabulary;
pub use ryo_execution_domain::domain::operator::types::Counting;
pub use ryo_execution_domain::domain::rule_intent::types::FrontierPurpose;
pub use ryo_execution_domain::domain::finish::types::Arena;
pub use ryo_execution_domain::domain::finish::types::PublicationReason;
pub use ryo_execution_domain::domain::operating::errors::OperatingError;
pub use ryo_execution_domain::domain::operating::types::SemanticLookup;
pub use ryo_execution_domain::domain::operator::types::LogicalCounts;
pub use ryo_execution_domain::domain::operator::types::RowsFed;
pub use ryo_execution_domain::domain::readiness::types::InvocationStateCount;
pub use ryo_execution_domain::domain::readiness::types::InvocationStateCounts;
pub use ryo_execution_domain::domain::readiness::types::ReadinessCounts;
pub use ryo_execution_domain::domain::readiness::types::ReadinessEvents;
pub use ryo_execution_domain::domain::row_emission::types::RowEmission;
pub use ryo_execution_domain::domain::rule_intent::types::ApplicabilityReason;
pub use ryo_execution_domain::domain::rule_intent::types::ApplicabilityRefusal;
pub use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
pub use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
pub use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
pub use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
pub use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
pub use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
pub use ryo_execution_domain::domain::shared_walk::types::WalkClassWork;
pub use ryo_execution_domain::domain::shared_walk::types::WalkCounts;
pub use ryo_execution_domain::domain::shared_walk::types::WalkDecodedRows;
pub use ryo_execution_domain::domain::shared_walk::types::WalkPassCount;
pub use ryo_execution_domain::domain::shared_walk::types::WalkPredicateRows;
pub use ryo_execution_domain::domain::shared_walk::types::WalkPredicateTests;
pub use ryo_execution_domain::domain::shared_walk::types::WalkSourceRows;
pub use ryo_execution_domain::domain::spelling::errors::SpellingError;
pub use ryo_execution_domain::domain::trust::types::RelationRead;

/// What `execution` offers its callers.
pub trait ExecutionUseCase {
    /// Begins a request execution over a prepared program and the corpus it runs on.
    fn execution(&self, program: &PreparedExecutionProgram, corpus: &[CorpusInput]) -> RequestExecution;
}
