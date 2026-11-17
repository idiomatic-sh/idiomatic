//! The support of the `witness` cases and of the groups nested in it.
use ryo_execution::inbound::use_cases::execution::interface::CaptureContext;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FactsWorker;
use ryo_execution::inbound::use_cases::execution::interface::FileRules;
use ryo_execution::inbound::use_cases::execution::interface::FileRunInput;
use ryo_execution::inbound::use_cases::execution::interface::Install;
use ryo_execution::inbound::use_cases::execution::interface::ObservationBatches;
use ryo_execution::inbound::use_cases::execution::interface::ObservationStage;
use ryo_execution::inbound::use_cases::execution::interface::ObserverPort;
use ryo_execution::inbound::use_cases::execution::interface::OccurrenceNumber;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::PhasePartials;
use ryo_execution::inbound::use_cases::execution::interface::PhaseRules;
use ryo_execution::inbound::use_cases::execution::interface::PhaseStop;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecutionProgram;
use ryo_execution::inbound::use_cases::execution::interface::ProgramObservationStagePort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramProjectPhasePort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProjectBatch;
use ryo_execution::inbound::use_cases::execution::interface::ProjectOutputOutcome;
use ryo_execution::inbound::use_cases::execution::interface::ProjectPhase;
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
use ryo_execution::inbound::use_cases::execution::interface::RequestExecution;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::interface::RuleOutcome;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::types::PrepareRequest;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::WorkerNumber;
use ryo_values::domain::{files::types::CorpusInput, stored_facts::types::ArenaBytes};

/// One variant per method of every entry trait, generated with its inventory so a variant cannot
/// be missing from `ALL`. `witness` below maps each trait method here, and the exhaustive `match`
/// in `controls::control` refuses to compile for a variant without a control.
macro_rules! methods {
    ($($method:ident),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub(in super::super) enum Method { $($method),+ }
        pub(in super::super) const ALL: &[Method] = &[$(Method::$method),+];
    };
}
methods!(
    UseCasePrepare,
    UseCaseExecution,
    RequestObservations,
    RequestFinish,
    RequestRunFile,
    RequestProjectPhase,
    RootsEvaluateRules,
    StageFinish,
    ObservedRunBatches,
);
/// Its methods never run: implementing every method of each entry trait is what the compiler checks.
pub(in super::super) struct Witness;

pub(in super::super) fn implements_every_entry_trait<
    W: ExecutionUseCase
        + PrepareUseCase
        + ProgramRequestExecutionPort
        + ProjectRootExecution
        + ProgramProjectPhasePort
        + ProgramObservationStagePort,
>(
    _: &W,
) {
}

pub(in super::super) fn witness(method: Method) -> ! {
    unreachable!("{method:?} is a compile-time witness only")
}

impl PrepareUseCase for Witness {
    fn prepare(&self, _: PrepareRequest) -> Result<PreparedExecutionProgram, OperatingError> {
        witness(Method::UseCasePrepare)
    }
}

impl ExecutionUseCase for Witness {
    fn execution(
        &self,
        _: &PreparedExecutionProgram,
        _: &[CorpusInput],
        _: ryo_execution::inbound::use_cases::execution::interface::Observing<dyn ObserverPort>,
    ) -> RequestExecution {
        witness(Method::UseCaseExecution)
    }
}

impl ProgramRequestExecutionPort for Witness {
    fn observations<'a>(
        &'a self,
        _: &'a mut ryo_execution::inbound::use_cases::execution::interface::ObservationArenas,
    ) -> Result<ObservationStage<'a>, OperatingError> {
        witness(Method::RequestObservations)
    }
    fn finish(self) {
        witness(Method::RequestFinish)
    }
    fn run_file(
        &self,
        _: FileRunInput<'_>,
        _: &mut Symbols,
        _: CaptureContext<'_>,
        _: &mut dyn FactsWorker,
    ) -> FileRules {
        witness(Method::RequestRunFile)
    }
    fn project_phase(&self) -> Result<ProjectPhase<'_>, OperatingError> {
        witness(Method::RequestProjectPhase)
    }
}

impl ProjectRootExecution for Witness {
    fn evaluate_rules<H, T, E>(
        &self,
        _: PhaseRules<'_>,
        _: &mut Symbols,
        _: impl FnMut(OccurrenceNumber) -> Result<H, E>,
        _: impl FnMut(OccurrenceNumber) -> Result<PhasePartials, E>,
        _: impl FnMut(RuleIndex) -> Result<(), E>,
        _: impl FnMut(
            OccurrenceNumber,
            RuleOutcome<ProjectOutputOutcome>,
            &[ArenaBytes],
            Option<WorkerNumber>,
            &mut Symbols,
        ) -> Result<T, E>,
    ) -> ProjectBatch<Result<T, PhaseStop<E>>, E> {
        witness(Method::RootsEvaluateRules)
    }
}

impl ProgramProjectPhasePort for Witness {
    fn finish(self) -> Result<(), OperatingError> {
        witness(Method::StageFinish)
    }
}

impl ProgramObservationStagePort for Witness {
    fn run_batches<H, T, E: From<OperatingError>>(
        self,
        _: ObservationBatches<'_>,
        _: &mut Symbols,
        _: impl FnMut(OccurrenceNumber) -> Result<H, E>,
        _: impl FnMut(OccurrenceNumber) -> Result<PhasePartials, E>,
        _: impl FnMut(RuleIndex) -> Result<(), E>,
        _: impl FnMut(
            OccurrenceNumber,
            RuleOutcome<ProjectOutputOutcome>,
            &[ArenaBytes],
            Option<WorkerNumber>,
            &mut Symbols,
        ) -> Result<T, E>,
        _: impl FnMut(OccurrenceNumber, Result<T, PhaseStop<E>>) -> Result<Option<Install>, E>,
    ) -> Result<(), E> {
        witness(Method::ObservedRunBatches)
    }
}

/// One attempt a control made: what it gave the method, and what the method answered, as debug text (the answers are of
/// several types: a preparation, a phase stop, a file end or an operating error).
pub(in super::super) type Probe = (String, String);

/// A probe of `what`, answered `answer`.
pub(in super::super) fn probe(what: impl Into<String>, answer: impl std::fmt::Debug) -> Probe {
    (what.into(), format!("{answer:?}"))
}
