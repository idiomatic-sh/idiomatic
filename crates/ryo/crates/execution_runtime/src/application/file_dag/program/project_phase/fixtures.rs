//! The programs, corpus and facts the project phase's own cases submit roots over, and the submission of a caller-built root set
//! through the crate-private root evaluation of a phase or a stage.
use crate::application::file_dag::frontiers::errors::PreparationError;
use crate::application::file_dag::frontiers::types::PreparedFrontiers;
use crate::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use crate::application::file_dag::program::observation_stage::interface::ObservationExecution as _;
use crate::application::file_dag::program::observation_stage::types::ObservationStage;
use crate::application::file_dag::program::project_phase::interface::ProjectStage as _;
use crate::application::file_dag::program::project_phase::types::{
    FrontierDemand, FrontierRequest, PhaseScope, ProjectInput, ProjectPhase, ProjectTask,
};
use crate::application::file_dag::program::rule_outcome::types::{ProjectBatch, ProjectOutputOutcome, RuleOutcome};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::{EmitDemand, ObservationDemand, RuleDemand};
use crate::application::file_dag::program::request_execution::interface::ExecutionRequest as _;
use crate::application::file_dag::program::request_execution::types::RequestExecution;
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::interface::{FactsFinish, FactsWorker};
use crate::outbound::facts::types::{ColumnsMut, KeptBucket, ResidentStability, WordsToKeep};
use ryo_execution_domain::domain::coverage::types::{Covered, InapplicableFiles, Partials, RefusedFiles, RuleIndex};
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, FileId, Relation};
use ryo_values::domain::files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds};
use ryo_values::domain::observations::types::{
    BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn, ObservationColumnName,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, ObservationBindingId, StepId, StepScope};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId};
use ryo_values::domain::records::types::ObservationReadinessLayout;
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
use ryo_values::domain::stored_facts::types::{Bucket, Digest};
use std::sync::Arc;

/// No compiled rule in these programs has a file frontier or a project relation.
pub(crate) struct NoFacts;
impl FactsFinish for NoFacts {
    fn kept(&self, _: Bucket) -> Result<KeptBucket, FactsError> {
        panic!("a submission control has no file frontier to fetch")
    }
}
impl FactsWorker for NoFacts {
    fn resident_stability(&self) -> ResidentStability {
        ResidentStability::Stable
    }
    fn rows(&self, _: Relation) -> Result<ryo_relations::domain::relations::types::ColumnsView<'_>, FactsError> {
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

/// The two places that hold a set of project roots.
#[derive(Clone, Copy)]
pub(crate) enum Entry {
    /// Ordinary roots through the request's project phase.
    Phase,
    /// Named roots through the observation stage that owns that phase.
    Stage,
}
/// One rule per file: each selects the first column of the files' functions and emits it.
fn plan() -> Plan {
    Plan {
        parameters: vec![],
        observation_inputs: vec![],
        observations: vec![],
        steps: vec![PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::Function),
                comparisons: vec![],
                kept: vec![ColumnIndex(0)],
            },
        }],
        comparison_origins: vec![],
        frontier: vec![],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
    }
}

fn rule(index: usize) -> RuleSpec {
    RuleSpec {
        id: RuleId(format!("public-boundary-{index}").into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId(format!("public-boundary-{index}").into()),
                text: "".into(),
            },
            modules: Default::default(),
        },
        applicability: RuleApplicability::Always,
        given: vec![],
        observations: vec![],
        spelled: RuleParameters("".into()),
        include: vec![],
        exclude: vec![],
    }
}

/// The prepared program of `rule_count` rules; the stage entry declares one closed named binding on rule 0.
pub(super) fn program(entry: Entry, rule_count: usize) -> PreparedExecutionProgram {
    let mut plans = vec![plan(); rule_count];
    let mut rules: Vec<RuleSpec> = (0..rule_count).map(rule).collect();
    let mut identity = ObservationIdentity::Independent;
    if matches!(entry, Entry::Stage) {
        // A named request needs a dependency schedule: rule 0 declares one closed empty binding.
        let columns = vec![ObservationColumn {
            name: ObservationColumnName("code".into()),
            kind: CellKind::Symbol,
        }];
        rules[0].observations.push(ObservationBinding {
            name: ObservationBindingName("codes".into()),
            columns: columns.clone(),
            producers: vec![],
        });
        plans[0].observation_inputs.push(BoundObservationInput {
            name: ObservationBindingName("codes".into()),
            columns,
            scope: StepScope::CrossFile,
        });
        plans[0].steps = vec![PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Select {
                from: Input::BoundObservation(ObservationBindingId(0)),
                comparisons: vec![],
                kept: vec![],
            },
        }];
        plans[0].frontier = vec![Input::BoundObservation(ObservationBindingId(0))];
        identity = ObservationIdentity::Named(ObservationReadinessLayout(Digest([7; 16])));
    }
    let digests: Vec<Digest> = (0..rule_count)
        .map(|index| Digest([u8::try_from(index).expect("a few rules"); 16]))
        .collect();
    let compiled = ryo_execution_domain::domain::prepared_request::service::assemble(plans, vec![vec![]; rule_count], &rules)
        .expect("valid authored plans");
    crate::application::file_dag::program::prepared_execution::service::prepare(
        compiled,
        &rules,
        &digests,
        &[],
        identity,
        Arc::new(crate::outbound::operators::factory::use_case(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(Arc::new(
                ryo_operators::outbound::matching::factory::use_case(),
            )),
        )),
        &crate::application::file_dag::fixtures::real_derived_port().into(),
        Arc::new(crate::outbound::digest::factory::blake3()),
    )
    .expect("valid public request")
}

/// One producer rule: a per-file emit, and a project observation of the files' module identities that only the facts' project
/// relation could answer; rule 1 binds that observation, so the producer exports it as a root.
pub(super) fn producer_program() -> PreparedExecutionProgram {
    use ryo_values::domain::observations::types::{ObservationName, ObservationProducer, PlanObservation};
    let relation = Relation::ModuleIdentity;
    let kinds = ryo_relations::domain::relations::fixtures::kinds(relation);
    let file = kinds.iter().position(|kind| *kind == CellKind::File).and_then(|at| u32::try_from(at).ok());
    let columns = vec![ObservationColumn { name: ObservationColumnName("file".into()), kind: CellKind::File }];
    let select = |from, kept| PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Select { from, comparisons: vec![], kept },
    };
    let mut produced = plan();
    produced.steps.push(select(Input::Relation(relation), vec![ColumnIndex(file.expect("a module identity row names its file"))]));
    let name = || ObservationName("files".into());
    let observation = PlanObservation { name: name(), source: Input::Step(StepId(1)), columns: columns.clone(), scope: StepScope::CrossFile };
    produced.observations.push(observation);
    let mut rules = [rule(0), rule(1)];
    let producers = vec![ObservationProducer::Output { rule: rules[0].id.clone(), output: name() }];
    let binding = || ObservationBindingName("files".into());
    rules[1].observations.push(ObservationBinding { name: binding(), columns: columns.clone(), producers });
    let mut consumer = plan();
    consumer.observation_inputs.push(BoundObservationInput { name: binding(), columns, scope: StepScope::CrossFile });
    consumer.steps = vec![select(Input::BoundObservation(ObservationBindingId(0)), vec![])];
    let compiled = ryo_execution_domain::domain::prepared_request::service::assemble(
        vec![produced, consumer],
        vec![vec![]; 2],
        &rules,
    )
    .expect("valid authored producer");
    crate::application::file_dag::program::prepared_execution::service::prepare(
        compiled,
        &rules,
        &[Digest([9; 16]), Digest([10; 16])],
        &[],
        ObservationIdentity::Named(ObservationReadinessLayout(Digest([11; 16]))),
        Arc::new(crate::outbound::operators::factory::use_case(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(Arc::new(
                ryo_operators::outbound::matching::factory::use_case(),
            )),
        )),
        &crate::application::file_dag::fixtures::real_derived_port().into(),
        Arc::new(crate::outbound::digest::factory::blake3()),
    )
    .expect("valid producer request")
}

pub(super) fn corpus() -> Vec<CorpusInput> {
    vec![CorpusInput::Available(CorpusEntry {
        executable: ExecutablePermission::Unknown,
        path: FilePath("input.py".into()),
        size: FileSize(3),
        modified: Nanoseconds(0),
    })]
}

pub(super) fn covered(rule_count: usize) -> [Covered; 1] {
    [Covered {
        file: FileId(0),
        rules: (0..rule_count).map(RuleIndex).collect(),
        partial: vec![],
    }]
}

/// Prepare the frontiers of `rule` against `scope`.
pub(super) fn prepare_for(request: &RequestExecution, scope: &PhaseScope, rule: RuleIndex) -> Result<PreparedFrontiers, PreparationError> {
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

pub(crate) enum Held<'a> {
    Phase(ProjectPhase<'a>),
    Stage(ObservationStage<'a>),
}
impl Held<'_> {
    fn phase(&self) -> &ProjectPhase<'_> {
        match self {
            Held::Phase(phase) => phase,
            Held::Stage(stage) => &stage.phase,
        }
    }
    fn scope(&self, rule: RuleIndex, corpus: &[CorpusInput], covered: &[Covered]) -> PhaseScope {
        self.phase()
            .project_scope(rule, corpus, covered, &[], &InapplicableFiles::default(), &RefusedFiles::default())
            .expect("a declared rule has a scope")
    }
    fn finish(self) {
        match self {
            Held::Phase(phase) => phase.finish().expect("the phase finishes"),
            Held::Stage(stage) => stage.finish_stage().expect("the stage finishes"),
        }
    }
}

/// What the caller declared: the scope and frontiers come from `declared`, while the task names `named`. The two differ only when
/// the caller submits a root nobody declared.
#[derive(Clone, Copy)]
pub(crate) struct Root {
    pub(crate) declared: usize,
    pub(crate) named: usize,
}
pub(crate) struct Submitted {
    pub(crate) outcomes: Vec<Result<RuleOutcome<ProjectOutputOutcome>, OperatingError>>,
    pub(crate) guarded: Vec<RuleIndex>,
    pub(crate) refused: Option<OperatingError>,
}
/// Evaluate `roots` through the real project phase of a request over `rule_count` rules; `refuse` is the rule the guard rejects,
/// if any.
pub(crate) fn submit(entry: Entry, rule_count: usize, roots: &[Root], refuse: Option<RuleIndex>) -> Submitted {
    let prepared = program(entry, rule_count);
    let corpus = corpus();
    let request = crate::application::file_dag::program::request_execution::service::begin(&prepared, &corpus);
    let covered = covered(rule_count);
    let mut arenas = Vec::new();
    let held = match entry {
        Entry::Phase => Held::Phase(request.project_phase().expect("the phase opens")),
        Entry::Stage => Held::Stage(request.observations(&mut arenas).expect("the stage opens")),
    };
    let mut symbols = Symbols::default();
    let partials = Partials::default();
    let scopes: Vec<PhaseScope> = roots
        .iter()
        .map(|root| held.scope(RuleIndex(root.declared), &corpus, &covered))
        .collect();
    let tasks = roots
        .iter()
        .zip(&scopes)
        .map(|(root, scope)| ProjectTask {
            rule: RuleIndex(root.named),
            demand: RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            input: ProjectInput {
                scope,
                evaluation: &partials,
                publication: &partials,
                ordinary: prepare_for(&request, scope, RuleIndex(root.declared)).expect("declared frontiers"),
                bound: &[],
                arenas: &[],
            },
        })
        .collect();
    let mut guarded = Vec::new();
    let batch = held.phase().evaluate(tasks, &mut symbols, |rule| {
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

/// A scope opened by one phase of the request, evaluated through another phase of it.
pub(crate) fn foreign_phase_outcome() -> Result<RuleOutcome<ProjectOutputOutcome>, OperatingError> {
    let prepared = program(Entry::Phase, 1);
    let corpus = corpus();
    let request = crate::application::file_dag::program::request_execution::service::begin(&prepared, &corpus);
    let covered = covered(1);
    let (owner, other) = (
        request.project_phase().expect("the first phase opens"),
        request.project_phase().expect("the second phase opens"),
    );
    let scope = owner
        .project_scope(RuleIndex(0), &corpus, &covered, &[], &InapplicableFiles::default(), &RefusedFiles::default())
        .expect("a declared rule has a scope");
    let ordinary = prepare_for(&request, &scope, RuleIndex(0)).expect("declared frontiers");
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
    owner.finish().expect("the first phase finishes");
    other.finish().expect("the second phase finishes");
    request.finish();
    match batch {
        ProjectBatch::Complete(mut done) => done.remove(0),
        ProjectBatch::Refused { error, .. } => Err(error),
    }
}
