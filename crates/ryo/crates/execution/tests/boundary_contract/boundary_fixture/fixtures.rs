//! The support of the `boundary_fixture` cases and of the groups nested in it.
use ryo_execution::inbound::use_cases::execution::interface::EmitDemand;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecutionProgram;
use ryo_execution::inbound::use_cases::execution::interface::ProjectBatch;
use ryo_execution::inbound::use_cases::execution::interface::ProjectOutputOutcome;
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
use ryo_execution::inbound::use_cases::execution::interface::RuleDemand;
use ryo_execution::inbound::use_cases::execution::interface::RuleOutcome;
use ryo_execution::inbound::use_cases::execution::interface::{
    CaptureContext, FactsWorker, FileRuleIntent, FileRules, FileStop, KindSupport, Partial,
    RanRule, RequestExecution, RootOutcome, SemanticSupport, SourceLanguage,
};
use ryo_execution::inbound::use_cases::execution::interface::{
    PhasePartials, PhaseRules, PhaseStop,
};
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::types::{PlanSource, PrepareRequest};
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_language::domain::elements::types::Elements;
use ryo_language::{domain::files::types::FilePath, domain::symbols::types::Symbols};
use ryo_relations::domain::relations::{
    fixtures::{kinds, node_id},
    types::{CellKind, FileId, Flag, Integer, Offset, Position, Relation, Span, Value},
};
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{ColumnIndex, Input, StepId, StepScope},
    plans::types::{Emit, Operation, Plan, PlanStep},
    query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
    rows::types::{RowCount, Table},
    rule_requests::types::{RuleParameters, RuleSpec},
    runs::types::{RuleApplicability, RuleId},
    stored_facts::types::Digest,
};
use std::sync::Arc;

pub(crate) fn engine() -> PrepareUseCaseImpl {
    PrepareUseCaseImpl::new(
        Arc::new(ryo_execution::inbound::use_cases::execution::fixtures::operators()),
        Arc::new(ryo_execution::inbound::use_cases::execution::fixtures::derived()).into(),
        Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
    )
}

pub(crate) fn prepared(engine: &PrepareUseCaseImpl) -> PreparedExecutionProgram {
    prepared_rules(engine, 1)
}

pub(crate) fn authored_request(count: usize) -> PrepareRequest {
    PrepareRequest {
        plans: PlanSource::Authored {
            plans: vec![
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
                };
                count
            ],
            bindings: vec![vec![]; count],
        },
        rules: (0..count)
            .map(|index| RuleSpec {
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
            })
            .collect(),
        digests: (0..count)
            .map(|index| {
                Digest([u8::try_from(index % 256).expect("a remainder of 256 fits in u8"); 16])
            })
            .collect(),
        hosted: vec![],
        observations: ObservationIdentity::Independent,
    }
}

pub(crate) fn prepared_rules(
    engine: &PrepareUseCaseImpl,
    count: usize,
) -> PreparedExecutionProgram {
    engine
        .prepare(authored_request(count))
        .expect("valid public request")
}

pub(crate) fn corpus() -> Vec<CorpusInput> {
    vec![CorpusInput::Available(CorpusEntry {
        executable: ExecutablePermission::Unknown,
        path: FilePath("input.py".into()),
        size: FileSize(3),
        modified: Nanoseconds(0),
    })]
}

pub(crate) fn source() -> Table {
    Table {
        kinds: kinds(Relation::Function).to_vec(),
        count: RowCount(1),
        cells: kinds(Relation::Function)
            .iter()
            .map(|kind| match kind {
                CellKind::Node => Value::Node(node_id(FileId(0), Position(2))),
                CellKind::Symbol => Value::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
                CellKind::File => Value::File(FileId(0)),
                CellKind::Span => Value::Span(Span {
                    start: Offset(0),
                    end: Offset(0),
                }),
                CellKind::Number => Value::Number(Integer(0)),
                CellKind::Flag => Value::Flag(Flag::Lowered),
            })
            .collect(),
    }
}

pub(crate) fn demand() -> RuleDemand {
    RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::NotRequired,
    }
}

/// Canonical Function rows for two matching declarations and one other declaration.
pub(crate) fn named_functions(symbols: &mut Symbols) -> Table {
    use ryo_language::domain::symbols::{fixtures::intern, types::ByteSlice};
    let main = intern(symbols, ByteSlice(b"mainother"), Offset(0), Offset(4));
    let other = intern(symbols, ByteSlice(b"mainother"), Offset(4), Offset(9));
    let mut table = source();
    let prototype = table.cells.clone();
    table.cells.clear();
    for (index, name) in [main, main, other].into_iter().enumerate() {
        let mut row = prototype.clone();
        row[0] = Value::Node(node_id(
            FileId(0),
            Position(u32::try_from(index).expect("a row position fits in u32")),
        ));
        row[1] = Value::Symbol(name);
        table.cells.extend(row);
    }
    table.count = RowCount(3);
    table
}

/// One occurrence of each of `phase.rules`, evaluated as one batch through the phase's one phase call, each over the same partials
/// (`evaluation` for the diagnostic, `publication` for what it publishes): the only route that opens a project session. Each
/// answer is the occurrence's outcome, or the engine's refusal of it.
pub(crate) fn project_roots<R: ProjectRootExecution>(
    root: &R,
    phase: PhaseRules<'_>,
    evaluation: &Partials,
    publication: &Partials,
    symbols: &mut Symbols,
) -> Vec<Result<RuleOutcome<ProjectOutputOutcome>, OperatingError>> {
    let batch = root.evaluate_rules(
        phase,
        symbols,
        |_| Ok::<(), OperatingError>(()),
        |_| {
            Ok(PhasePartials {
                evaluation: evaluation.clone(),
                publication: publication.clone(),
            })
        },
        |_| Ok(()),
        |_, outcome, _, _, _| Ok(outcome),
    );
    let ProjectBatch::Complete(done) = batch else {
        panic!("the guard admits every occurrence")
    };
    done.into_iter()
        .map(|each| match each {
            Ok(outcome) => Ok(outcome),
            Err(PhaseStop::Operating(error)) => Err(error),
            Err(other) => panic!("the occurrence is scoped and its frontiers prepared: {other:?}"),
        })
        .collect()
}

/// The one occurrence of `phase` through [`project_roots`]; `ShapeMismatch` when the batch does not answer it exactly once.
pub(crate) fn project_root<R: ProjectRootExecution>(
    root: &R,
    phase: PhaseRules<'_>,
    evaluation: &Partials,
    publication: &Partials,
    symbols: &mut Symbols,
) -> Result<RuleOutcome<ProjectOutputOutcome>, OperatingError> {
    // One occurrence has one answer: a batch that answers it other than once is refused as a mismatched shape.
    <[_; 1]>::try_from(project_roots(root, phase, evaluation, publication, symbols))
        .map_or(Err(OperatingError::ShapeMismatch), |[one]| one)
}

/// The authored plans of a request a control builds, for the control to edit before it prepares.
pub(crate) fn plans(request: &mut PrepareRequest) -> &mut Vec<Plan> {
    match &mut request.plans {
        PlanSource::Authored { plans, .. } => plans,
        PlanSource::Compiled(_) => panic!("a control edits only the plans it authored"),
    }
}

/// The authored bindings of a request a control builds, for the control to edit before it prepares.
pub(crate) fn bindings(
    request: &mut PrepareRequest,
) -> &mut Vec<Vec<ryo_values::domain::query_bindings::types::ParameterBinding>> {
    match &mut request.plans {
        PlanSource::Authored { bindings, .. } => bindings,
        PlanSource::Compiled(_) => panic!("a control edits only the bindings it authored"),
    }
}

/// The authored plans of a request a control reads.
pub(crate) fn plans_of(request: &PrepareRequest) -> &[Plan] {
    match &request.plans {
        PlanSource::Authored { plans, .. } => plans,
        PlanSource::Compiled(_) => panic!("a control reads only the plans it authored"),
    }
}

/// The readiness a finished file observed, which a control that captures requires.
pub(crate) trait Observed {
    /// The observed readiness; a file that observed none fails the control with `reason`.
    fn observed(self, reason: &str) -> Arc<ryo_values::domain::records::types::FrontierReadiness>;
}

impl Observed for ryo_execution::inbound::use_cases::execution::interface::FinishedReadiness {
    fn observed(self, reason: &str) -> Arc<ryo_values::domain::records::types::FrontierReadiness> {
        match self {
            Self::Observed(readiness) => readiness,
            Self::Unobserved => panic!("{reason}"),
        }
    }
}

/// The authored plans and bindings of a request together, for a control that edits both in one pass.
pub(crate) fn authored_parts(
    request: &mut PrepareRequest,
) -> (
    &mut Vec<Plan>,
    &mut Vec<Vec<ryo_values::domain::query_bindings::types::ParameterBinding>>,
) {
    match &mut request.plans {
        PlanSource::Authored { plans, bindings } => (plans, bindings),
        PlanSource::Compiled(_) => panic!("a control edits only the request it authored"),
    }
}

/// What one file is run with through the one per-file route, `run_file`: file 0, recognized, over the source `abc`, read in
/// full, with no semantic lookup and no prepared kinds, unless a control says otherwise.
pub(crate) struct FileRun<'a> {
    pub(crate) language: SourceLanguage,
    pub(crate) file: FileId,
    pub(crate) evaluation: &'a [Partial],
    pub(crate) publication: &'a [Partial],
    pub(crate) intents: &'a [FileRuleIntent],
    pub(crate) source: &'a [u8],
    pub(crate) elements: &'a Elements,
    pub(crate) semantic: SemanticSupport<'a>,
    pub(crate) kinds: KindSupport,
    /// The first frontier bucket of each rule; every rule's is bucket 0 when empty.
    pub(crate) first: &'a [Bucket],
}

impl<'a> FileRun<'a> {
    /// File 0 run with `intents` over the tree `elements`.
    pub(crate) fn of(intents: &'a [FileRuleIntent], elements: &'a Elements) -> Self {
        Self {
            language: SourceLanguage::Recognized,
            file: FileId(0),
            evaluation: &[],
            publication: &[],
            intents,
            source: b"abc",
            elements,
            semantic: SemanticSupport::Absent,
            kinds: KindSupport::Absent,
            first: &[],
        }
    }
}

/// `run` through `request`'s one per-file route over `facts`, its finished rows written to a fresh capture context of worker 0.
pub(crate) fn run_file(
    request: &RequestExecution,
    run: FileRun<'_>,
    symbols: &mut Symbols,
    facts: &mut dyn FactsWorker,
) -> FileRules {
    use ryo_execution::inbound::use_cases::execution::interface::Arena;
    use ryo_relations::domain::relations::types::{Words, WorkerNumber};
    let (mut arena, mut places, mut words) = (Vec::new(), Vec::new(), Words::default());
    run_file_into(
        request,
        run,
        symbols,
        CaptureContext {
            worker: WorkerNumber(0),
            arena: Arena(&mut arena),
            places: &mut places,
            words: &mut words,
        },
        facts,
    )
}

/// `run` through `request`'s one per-file route over `facts`, its finished rows written to `capture`.
pub(crate) fn run_file_into(
    request: &RequestExecution,
    run: FileRun<'_>,
    symbols: &mut Symbols,
    capture: CaptureContext<'_>,
    facts: &mut dyn FactsWorker,
) -> FileRules {
    use ryo_execution::inbound::use_cases::execution::fixtures::partiality;
    use ryo_execution::inbound::use_cases::execution::interface::{
        FileRunInput, ProgramRequestExecutionPort as _,
    };
    use ryo_execution_runtime::outbound::facts::types::fact_sources::value::FactSources;
    let first = if run.first.is_empty() {
        vec![Bucket(0); run.intents.len()]
    } else {
        run.first.to_vec()
    };
    // The file's partiality is the run's own: the partial reads given are what core said of the file.
    let (said, answering) = partiality(run.evaluation, run.publication);
    request.run_file(
        FileRunInput {
            language: run.language,
            file: run.file,
            intents: run.intents,
            source: ryo_values::domain::files::types::SourceView(run.source),
            sources: FactSources::unread(run.elements),
            wanted: &[],
            active: &[],
            said: &said,
            recovery: ryo_execution::inbound::use_cases::metadata::interface::Recovery::Whole,
            answering,
            semantic: run.semantic,
            kinds: run.kinds,
            first: &first,
        },
        symbols,
        capture,
        facts,
    )
}

/// What a rule's run came to at its emitted root: the root it delivered, or what stopped it.
pub(crate) fn root(ran: RanRule) -> Result<RootOutcome, FileStop> {
    use ryo_execution::inbound::use_cases::execution::interface::{EmittedRoot, RuleAfter};
    match (ran.emit, ran.after) {
        (EmittedRoot::Delivered(outcome), _) => Ok(outcome),
        (EmittedRoot::Withheld, RuleAfter::Stopped(stop)) => Err(stop),
        (EmittedRoot::Withheld, RuleAfter::Ran { .. } | RuleAfter::Unreached) => {
            Err(FileStop::Operating(OperatingError::NotRun))
        }
    }
}

/// The rule `rule` of `rules`, which ran.
pub(crate) fn ran(rules: &mut FileRules, rule: usize) -> RanRule {
    let at = rules
        .rules
        .iter()
        .position(|each| each.rule.0 == rule)
        .unwrap_or_else(|| panic!("rule {rule} ran"));
    rules.rules.remove(at)
}
