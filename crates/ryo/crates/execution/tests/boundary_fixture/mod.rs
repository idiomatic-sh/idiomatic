//! Canonical public request and catalog-row assembly shared by boundary controls.
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::application::file_dag::program::project_phase::interface::ProjectRootExecution;
use ryo_execution::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution::application::file_dag::program::project_phase::types::ProjectInput;
use ryo_execution::application::file_dag::program::project_phase::types::ProjectTask;
use ryo_execution::application::file_dag::program::rule_outcome::types::ProjectBatch;
use ryo_execution::application::file_dag::program::rule_outcome::types::RuleOutcome;
use ryo_execution::application::file_dag::program::rule_outcome::types::ProjectOutputOutcome;
use ryo_execution::inbound::use_cases::execution::interface::EmitDemand;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::RuleDemand;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution::inbound::use_cases::prepare::types::{PlanSource, PrepareRequest};
use ryo_language::{domain::files::types::FilePath, domain::symbols::types::Symbols};
use ryo_relations::domain::relations::{
    fixtures::{kinds, node_id},
    types::{CellKind, FileId, Flag, Integer, Offset, Position, Relation, Span, Value},
};
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
use std::{collections::BTreeMap, sync::Arc};
pub(crate) fn engine() -> PrepareUseCaseImpl {
    PrepareUseCaseImpl::new(
        Arc::new(
            ryo_execution::outbound::operators::factory::use_case(ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case()))),
        ),
        Arc::new(ryo_execution::outbound::derived::factory::semantics()),
    )
}
pub(crate) fn prepared(engine: &PrepareUseCaseImpl) -> PreparedExecutionProgram {
    prepared_rules(engine, 1)
}
pub(crate) fn authored_request(count: usize) -> PrepareRequest {
    PrepareRequest { plans: PlanSource::Authored { plans: vec![
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
        ], bindings: vec![vec![]; count] }, rules: (0..count)
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
            .collect(), digests: (0..count).map(|index| Digest([index as u8; 16])).collect(), hosted: vec![], observations: ObservationIdentity::Independent }
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
        row[0] = Value::Node(node_id(FileId(0), Position(index as u32)));
        row[1] = Value::Symbol(name);
        table.cells.extend(row);
    }
    table.count = RowCount(3);
    table
}

/// The inputs are declared roots for one rule, evaluated as one batch through the phase or
/// stage: the only route that opens a project session. Identical roots share one invocation.
pub(crate) fn project_roots<R: ProjectRootExecution>(
    root: &R,
    rule: RuleIndex,
    demand: RuleDemand,
    inputs: Vec<ProjectInput<'_>>,
    symbols: &mut Symbols,
) -> Vec<Result<RuleOutcome<ProjectOutputOutcome>, OperatingError>> {
    let tasks = inputs
        .into_iter()
        .map(|input| ProjectTask {
            rule,
            demand,
            input,
        })
        .collect();
    match root.evaluate(tasks, symbols, |_| Ok::<(), OperatingError>(())) {
        ProjectBatch::Complete(done) => done,
        ProjectBatch::Refused { error, .. } => panic!("the guard admits every root: {error:?}"),
    }
}
/// One declared root through [`project_roots`].
pub(crate) fn project_root<R: ProjectRootExecution>(
    root: &R,
    rule: RuleIndex,
    demand: RuleDemand,
    input: ProjectInput<'_>,
    symbols: &mut Symbols,
) -> Result<RuleOutcome<ProjectOutputOutcome>, OperatingError> {
    project_roots(root, rule, demand, vec![input], symbols).remove(0)
}

/// The authored plans of a request a control builds, for the control to edit before it prepares.
pub(crate) fn plans(request: &mut PrepareRequest) -> &mut Vec<ryo_values::domain::plans::types::Plan> {
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
pub(crate) fn plans_of(request: &PrepareRequest) -> &[ryo_values::domain::plans::types::Plan] {
    match &request.plans {
        PlanSource::Authored { plans, .. } => plans,
        PlanSource::Compiled(_) => panic!("a control reads only the plans it authored"),
    }
}

/// The readiness a finished file observed, which a control that captures requires.
pub(crate) trait Observed {
    /// The observed readiness; a file that observed none fails the control with `reason`.
    fn observed(self, reason: &str) -> std::sync::Arc<ryo_values::domain::records::types::FrontierReadiness>;
}

impl Observed for ryo_execution::application::file_dag::program::file_session::types::FinishedReadiness {
    fn observed(self, reason: &str) -> std::sync::Arc<ryo_values::domain::records::types::FrontierReadiness> {
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
    &mut Vec<ryo_values::domain::plans::types::Plan>,
    &mut Vec<Vec<ryo_values::domain::query_bindings::types::ParameterBinding>>,
) {
    match &mut request.plans {
        PlanSource::Authored { plans, bindings } => (plans, bindings),
        PlanSource::Compiled(_) => panic!("a control edits only the request it authored"),
    }
}
