//! One project submission through the public phase, with the roots its guard was asked about.
use ryo_execution_runtime::outbound::facts::types::ColumnsMut;
use ryo_execution_runtime::outbound::facts::errors::FactsError;
use ryo_execution_runtime::outbound::facts::interface::FactsFinish;
use ryo_execution_runtime::outbound::facts::interface::FactsWorker;
use ryo_execution_runtime::outbound::facts::types::KeptBucket;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_execution_runtime::outbound::facts::types::WordsToKeep;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use crate::boundary_fixture::corpus;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::ObservationExecution;
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution_runtime::application::file_dag::frontiers::errors::PreparationError;
use ryo_execution_runtime::application::file_dag::frontiers::types::PreparedFrontiers;
use ryo_execution_runtime::application::file_dag::program::observation_stage::types::ObservationStage;
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::ProjectInput;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::ProjectPhase;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::PhaseScope;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::ProjectTask;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::FrontierDemand;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::FrontierRequest;
use ryo_execution_runtime::application::file_dag::program::request_execution::types::RequestExecution;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::ProjectBatch;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RuleOutcome;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::ProjectOutputOutcome;
use ryo_execution::inbound::use_cases::execution::interface::Covered;
use ryo_execution::inbound::use_cases::execution::interface::EmitDemand;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::RuleDemand;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, FileId};
use ryo_values::domain::{
    files::types::CorpusInput,
    observations::types::{
        BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn,
        ObservationColumnName,
    },
    plan_parts::types::{Input, ObservationBindingId, StepScope},
    plans::types::{Operation, PlanStep},
    records::types::ObservationReadinessLayout,
    stored_facts::types::{Bucket, Digest},
};

/// No compiled rule in these programs has a file frontier or a project relation.
struct NoFacts;
impl FactsFinish for NoFacts {
    fn kept(&self, _: Bucket) -> Result<KeptBucket, FactsError> {
        panic!("a submission control has no file frontier to fetch")
    }
}
impl FactsWorker for NoFacts {
    fn resident_stability(&self) -> ResidentStability {
        ResidentStability::Stable
    }
    fn rows(
        &self,
        _: ryo_relations::domain::relations::types::Relation,
    ) -> Result<ryo_relations::domain::relations::types::ColumnsView<'_>, FactsError> {
        panic!("a submission control has no project relation to read")
    }
    fn columns_mut(&mut self) -> Result<ColumnsMut<'_>, FactsError> {
        panic!("a submission control writes no facts")
    }
    fn keep(&self, _: Bucket, _: WordsToKeep<'_>) -> Result<(), FactsError> {
        panic!("a submission control writes no facts")
    }
    fn release(&mut self) -> Result<(), FactsError> {
        panic!("a submission control writes no facts")
    }
    fn finished(self: Box<Self>) -> Result<Box<dyn FactsFinish>, FactsError> {
        panic!("a submission control writes no facts")
    }
}

/// The two public entries that accept a set of project roots.
#[derive(Clone, Copy)]
pub(crate) enum Entry {
    /// Ordinary roots through the request's project phase.
    Phase,
    /// Named roots through the observation stage that owns that phase.
    Stage,
}
pub(crate) enum Held<'a> {
    Phase(ProjectPhase<'a>),
    Stage(ObservationStage<'a>),
}
impl Held<'_> {
    pub(crate) fn try_scope(
        &self,
        rule: RuleIndex,
        corpus: &[CorpusInput],
        covered: &[Covered],
    ) -> Result<PhaseScope, OperatingError> {
        match self {
            Held::Phase(phase) => phase.project_scope(
                rule,
                corpus,
                covered,
                &[],
                &ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default(),
            ),
            Held::Stage(stage) => stage.project_scope(
                rule,
                corpus,
                covered,
                &[],
                &ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default(),
            ),
        }
    }
    fn scope(&self, rule: RuleIndex, corpus: &[CorpusInput], covered: &[Covered]) -> PhaseScope {
        self.try_scope(rule, corpus, covered).unwrap()
    }
    fn evaluate<E>(
        &self,
        tasks: Vec<ProjectTask<'_>>,
        symbols: &mut Symbols,
        guard: impl FnMut(RuleIndex) -> Result<(), E>,
    ) -> ProjectBatch<Result<RuleOutcome<ProjectOutputOutcome>, OperatingError>, E> {
        match self {
            Held::Phase(phase) => phase.evaluate(tasks, symbols, guard),
            Held::Stage(stage) => stage.evaluate(tasks, symbols, guard),
        }
    }
    fn finish(self) {
        match self {
            Held::Phase(phase) => phase.finish().unwrap(),
            Held::Stage(stage) => stage.finish_stage().unwrap(),
        }
    }
}

/// The prepared program of `rule_count` rules; the stage entry declares one closed named binding.
fn program(
    engine: &PrepareUseCaseImpl,
    entry: Entry,
    rule_count: usize,
) -> PreparedExecutionProgram {
    let mut authored = crate::boundary_fixture::authored_request(rule_count);
    if matches!(entry, Entry::Stage) {
        // A named request needs a dependency schedule: rule 0 declares one closed empty binding.
        let columns = vec![ObservationColumn {
            name: ObservationColumnName("code".into()),
            kind: CellKind::Symbol,
        }];
        authored.rules[0].observations.push(ObservationBinding {
            name: ObservationBindingName("codes".into()),
            columns: columns.clone(),
            producers: vec![],
        });
        crate::boundary_fixture::plans(&mut authored)[0]
            .observation_inputs
            .push(BoundObservationInput {
                name: ObservationBindingName("codes".into()),
                columns,
                scope: StepScope::CrossFile,
            });
        crate::boundary_fixture::plans(&mut authored)[0].steps = vec![PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Select {
                from: Input::BoundObservation(ObservationBindingId(0)),
                comparisons: vec![],
                kept: vec![],
            },
        }];
        crate::boundary_fixture::plans(&mut authored)[0].frontier = vec![Input::BoundObservation(ObservationBindingId(0))];
        authored.observations =
            ObservationIdentity::Named(ObservationReadinessLayout(Digest([7; 16])));
    }
    engine.prepare(authored).expect("valid public request")
}
/// What the caller declared: the scope and frontiers come from `declared`, while the
/// task names `named`. The two differ only when the caller submits a root nobody declared.
#[derive(Clone, Copy)]
pub(crate) struct Root {
    pub declared: usize,
    pub named: usize,
}
pub(crate) struct Submitted {
    pub outcomes: Vec<Result<RuleOutcome<ProjectOutputOutcome>, OperatingError>>,
    pub guarded: Vec<RuleIndex>,
    pub refused: Option<OperatingError>,
}
/// Evaluate `roots` through the real project phase of a request over `rule_count` rules.
/// `refuse` is the rule the guard rejects, if any.
pub(crate) fn submit(
    engine: &PrepareUseCaseImpl,
    entry: Entry,
    rule_count: usize,
    roots: &[Root],
    refuse: Option<RuleIndex>,
) -> Submitted {
    let prepared = program(engine, entry, rule_count);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus);
    let covered = [Covered {
        file: FileId(0),
        rules: (0..rule_count).map(RuleIndex).collect(),
        partial: vec![],
    }];
    let mut arenas = Vec::new();
    let held = match entry {
        Entry::Phase => Held::Phase(request.project_phase().unwrap()),
        Entry::Stage => Held::Stage(request.observations(&mut arenas).unwrap()),
    };
    let project_symbols = Symbols::default();
    let mut symbols = Symbols::default();
    let partials = Partials::default();
    let mut scopes = Vec::new();
    let mut frontiers = Vec::new();
    for root in roots {
        let scope = held.scope(RuleIndex(root.declared), &corpus, &covered);
        frontiers.push(
            request
                .prepare_frontiers(
                    FrontierRequest {
                        demand: FrontierDemand::Diagnostic,
                        scope: &scope,
                        rule: RuleIndex(root.declared),
                        first: Bucket(0),
                        facts: &NoFacts,
                        project: &NoFacts,
                        project_symbols: &project_symbols,
                    },
                    &mut symbols,
                )
                .unwrap(),
        );
        scopes.push(scope);
    }
    let tasks = roots
        .iter()
        .zip(&scopes)
        .zip(frontiers)
        .map(|((root, scope), ordinary)| ProjectTask {
            rule: RuleIndex(root.named),
            demand: RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            input: ProjectInput {
                scope,
                evaluation: &partials,
                publication: &partials,
                ordinary,
                bound: &[],
                arenas: &[],
            },
        })
        .collect();
    let mut guarded = Vec::new();
    let batch = held.evaluate(tasks, &mut symbols, |rule| {
        guarded.push(rule);
        if Some(rule) == refuse {
            Err(OperatingError::WrongBinding)
        } else {
            Ok(())
        }
    });
    let (outcomes, refused) = match batch {
        ProjectBatch::Complete(outcomes) => (outcomes, None),
        ProjectBatch::Refused { completed, error } => (completed, Some(error)),
    };
    drop(scopes);
    held.finish();
    request.finish();
    Submitted {
        outcomes,
        guarded,
        refused,
    }
}

/// Open the request and its phase (or observation stage) for `entry`, hand them to `check`,
/// and release everything once `check` has dropped every scope it made.
pub(crate) fn with_held<T>(
    engine: &PrepareUseCaseImpl,
    entry: Entry,
    rule_count: usize,
    check: impl FnOnce(&RequestExecution, &mut Held<'_>, &[CorpusInput], &[Covered]) -> T,
) -> T {
    let prepared = program(engine, entry, rule_count);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus);
    let covered = [Covered {
        file: FileId(0),
        rules: (0..rule_count).map(RuleIndex).collect(),
        partial: vec![],
    }];
    let mut arenas = Vec::new();
    let mut held = match entry {
        Entry::Phase => Held::Phase(request.project_phase().unwrap()),
        Entry::Stage => Held::Stage(request.observations(&mut arenas).unwrap()),
    };
    let found = check(&request, &mut held, &corpus, &covered);
    held.finish();
    request.finish();
    found
}
/// Prepare the frontiers of `rule` against a scope that declared another rule.
pub(crate) fn prepare_for(
    request: &RequestExecution,
    scope: &PhaseScope,
    rule: RuleIndex,
) -> Result<PreparedFrontiers, PreparationError> {
    request.prepare_frontiers(
        FrontierRequest {
            demand: FrontierDemand::Diagnostic,
            scope,
            rule,
            first: Bucket(0),
            facts: &NoFacts,
            project: &NoFacts,
            project_symbols: &Symbols::default(),
        },
        &mut Symbols::default(),
    )
}
/// A scope opened by one phase of the request, evaluated through another phase of it.
pub(crate) fn foreign_phase_outcome(
    engine: &PrepareUseCaseImpl,
) -> Result<RuleOutcome<ProjectOutputOutcome>, OperatingError> {
    let prepared = program(engine, Entry::Phase, 1);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus);
    let covered = [Covered {
        file: FileId(0),
        rules: vec![RuleIndex(0)],
        partial: vec![],
    }];
    let (owner, other) = (
        request.project_phase().unwrap(),
        request.project_phase().unwrap(),
    );
    let scope = owner
        .project_scope(
            RuleIndex(0),
            &corpus,
            &covered,
            &[],
            &ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default(),
        )
        .unwrap();
    let ordinary = prepare_for(&request, &scope, RuleIndex(0)).unwrap();
    let partials = Partials::default();
    let batch = other.evaluate(
        vec![ProjectTask {
            rule: RuleIndex(0),
            demand: RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            input: ProjectInput {
                scope: &scope,
                evaluation: &partials,
                publication: &partials,
                ordinary,
                bound: &[],
                arenas: &[],
            },
        }],
        &mut Symbols::default(),
        |_| Ok::<(), OperatingError>(()),
    );
    drop(scope);
    owner.finish().unwrap();
    other.finish().unwrap();
    request.finish();
    match batch {
        ProjectBatch::Complete(mut done) => done.remove(0),
        ProjectBatch::Refused { error, .. } => Err(error),
    }
}

/// A compiled rule with no file frontier to fetch.
pub(crate) struct NoFileFrontiers;
impl FactsFinish for NoFileFrontiers {
    fn kept(&self, _: Bucket) -> Result<KeptBucket, FactsError> {
        panic!("this compiled rule has no file frontier to fetch")
    }
}
