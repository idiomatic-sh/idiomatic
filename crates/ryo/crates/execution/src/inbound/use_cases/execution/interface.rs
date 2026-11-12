//! What the `execution` use case promises its callers, in the core's own vocabulary.
use ryo_values::domain::files::types::CorpusInput;

// The application handles and protocol traits the contract is made of.
pub use ryo_execution_runtime::application::file_dag::frontiers::errors::FrontiersError;
pub use ryo_execution_runtime::application::file_dag::program::file_session::interface::ProgramFileSessionPort;
pub use ryo_execution_runtime::application::file_dag::program::file_session::types::CaptureContext;
pub use ryo_execution_runtime::application::file_dag::program::file_session::types::EmittedRoot;
pub use ryo_execution_runtime::application::file_dag::program::file_session::types::FileEnd;
pub use ryo_execution_runtime::application::file_dag::program::file_session::types::FileFinished;
#[cfg(any(test, feature = "fixtures"))]
pub use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
pub use ryo_execution_runtime::application::file_dag::program::file_session::types::FileRules;
pub use ryo_execution_runtime::application::file_dag::program::file_session::types::FileRunInput;
pub use ryo_execution_runtime::application::file_dag::program::file_session::types::FileSession;
pub use ryo_execution_runtime::application::file_dag::program::file_session::types::FileStop;
pub use ryo_execution_runtime::application::file_dag::program::file_session::types::FinishedReadiness;
pub use ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport;
pub use ryo_execution_runtime::application::file_dag::program::file_session::types::RanRule;
pub use ryo_execution_runtime::application::file_dag::program::file_session::types::RuleAfter;
pub use ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport;
pub use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
#[cfg(any(test, feature = "fixtures"))]
pub use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
pub use ryo_execution_runtime::application::file_dag::program::observation_stage::interface::ProgramObservationStagePort;
pub use ryo_execution_runtime::application::file_dag::program::observation_stage::types::AdmittedOutput;
pub use ryo_execution_runtime::application::file_dag::program::observation_stage::types::BatchNumber;
pub use ryo_execution_runtime::application::file_dag::program::observation_stage::types::BatchOccurrence;
pub use ryo_execution_runtime::application::file_dag::program::observation_stage::types::Install;
pub use ryo_execution_runtime::application::file_dag::program::observation_stage::types::ObservationBatches;
pub use ryo_execution_runtime::application::file_dag::program::observation_stage::types::ObservationStage;
pub use ryo_execution_runtime::application::file_dag::program::observed_publication::service::ObservedPublication;
pub use ryo_execution_runtime::application::file_dag::program::prepared_execution::interface::PreparedReporting;
pub use ryo_execution_runtime::application::file_dag::program::prepared_execution::interface::ProgramPreparedExecutionPort;
pub use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::Demand;
pub use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::NamedLayout;
pub use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::ObservationContract;
pub use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
pub use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::ReplayClosure;
pub use ryo_execution_runtime::application::file_dag::program::project_phase::interface::ProgramProjectPhasePort;
pub use ryo_execution_runtime::application::file_dag::program::project_phase::interface::ProjectRootExecution;
pub use ryo_execution_runtime::application::file_dag::program::project_phase::types::OccurrenceNumber;
pub use ryo_execution_runtime::application::file_dag::program::project_phase::types::PhasePartials;
pub use ryo_execution_runtime::application::file_dag::program::project_phase::types::PhaseRules;
pub use ryo_execution_runtime::application::file_dag::program::project_phase::types::PhaseStop;
pub use ryo_execution_runtime::application::file_dag::program::project_phase::types::ProjectPhase;
pub use ryo_execution_runtime::application::file_dag::program::request_execution::interface::ProgramRequestExecutionPort;
pub use ryo_execution_runtime::application::file_dag::program::request_execution::types::RequestExecution;
pub use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::ProjectBatch;
pub use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::ProjectObservation;
pub use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::ProjectOutputOutcome;
pub use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
pub use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RuleOutcome;
pub use ryo_execution_runtime::outbound::facts::interface::FactsWorker;
pub use ryo_execution_runtime::outbound::observer::interface::ObserverPort;
// Canonical inbound signature vocabulary from private pure concepts.
pub use ryo_execution_domain::domain::coverage::types::Covered;
pub use ryo_execution_domain::domain::coverage::types::FileObservationReadiness;
pub use ryo_execution_domain::domain::coverage::types::FilePartial;
pub use ryo_execution_domain::domain::coverage::types::InapplicableFiles;
pub use ryo_execution_domain::domain::coverage::types::Partial;
pub use ryo_execution_domain::domain::coverage::types::Partials;
pub use ryo_execution_domain::domain::coverage::types::RefusedFiles;
pub use ryo_execution_domain::domain::coverage::types::RuleIndex;
pub use ryo_execution_domain::domain::finish::types::Arena;
pub use ryo_execution_domain::domain::frontier_capture::types::CapturedFrontier;
pub use ryo_execution_domain::domain::frontier_capture::types::FrontierCapture;
pub use ryo_execution_domain::domain::observation_schedule::types::ObservedFrontierRoot;
pub use ryo_execution_domain::domain::operating::errors::OperatingError;
pub use ryo_execution_domain::domain::operating::types::SemanticLookup;
pub use ryo_execution_domain::domain::row_emission::types::RowEmission;
pub use ryo_execution_domain::domain::rule_intent::errors::RuleIntentError;
pub use ryo_execution_domain::domain::rule_intent::types::ApplicabilityReason;
pub use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
pub use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
pub use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
pub use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
pub use ryo_execution_domain::domain::rule_intent::types::FrontierPurpose;
pub use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
pub use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
pub use ryo_execution_domain::domain::rule_intent::types::SemanticNeed;
pub use ryo_execution_domain::domain::trust::types::RelationRead;

/// What `execution` offers its callers.
pub trait ExecutionUseCase {
    /// Begins a request execution over a prepared program and the corpus it runs on, telling `observer` of the engine's work
    /// when the caller asks for it.
    fn execution(
        &self,
        program: &PreparedExecutionProgram,
        corpus: &[CorpusInput],
        observer: Option<std::sync::Arc<dyn ObserverPort>>,
    ) -> RequestExecution;
}
