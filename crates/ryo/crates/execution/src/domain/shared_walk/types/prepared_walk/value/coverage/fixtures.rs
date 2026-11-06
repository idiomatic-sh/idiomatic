//! Plans use the normal operator, graph and physical-program compilers.
// ignore: idiomatic.arch/file-length - the corpus holds one plan per Operation variant beside the shapes; splitting would separate each plan from the meaning checked against it
use crate::domain::{
    coverage::types::RuleIndex,
    executable_program::types::{
        ExecutableProgram, InputSource, PartitionRequirement, ProducerPartitions,
    },
    operating::errors::OperatingError,
    operator::contract::types::RequestOperators,
};
use ryo_relations::domain::relations::{service::ALL, types::Relation};
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation},
    plan_parts::types::{
        CapturePattern, ColumnIndex, Comparison, Direction, HostedNumber, Input, JoinColumn,
        LiteralText, Operand, ParameterId, ParameterName, QueryLiteral, RepeatedRows, Side,
        StepId, TableKey, Test,
    },
    plans::{
        service::kind_of,
        types::{
            Emit, Operation, OperationKind, ParameterType, Plan, PlanParameter, PlanStep,
            StepScope,
        },
    },
    query_bindings::types::{BindingCell, BindingTable, ParameterBinding},
};

struct FileSources;
impl ProducerPartitions for FileSources {
    fn hosted(
        &self,
        _: RuleIndex,
        _: HostedNumber,
    ) -> Result<PartitionRequirement, OperatingError> {
        Err(OperatingError::NotRead)
    }
    fn parameter(
        &self,
        _: RuleIndex,
        _: ParameterId,
    ) -> Result<PartitionRequirement, OperatingError> {
        Err(OperatingError::Unbound)
    }
    fn relation(&self, _: Relation) -> PartitionRequirement {
        PartitionRequirement::CurrentFile
    }
    fn observation(
        &self,
        _: RuleIndex,
        _: ryo_values::domain::observations::types::ObservationBindingId,
    ) -> Result<PartitionRequirement, OperatingError> {
        Err(OperatingError::NotRead)
    }
}
pub(super) fn plan(operation: Operation, scope: StepScope) -> Plan {
    Plan {
        observation_inputs: vec![],
        parameters: vec![],
        comparison_origins: vec![],
        observations: vec![],
        steps: vec![PlanStep { operation, scope }],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
    }
}
pub(super) struct CompiledCorpus {
    pub(super) program: ExecutableProgram,
    pub(super) walk: super::super::PreparedWalk,
}
pub(super) fn compiled() -> CompiledCorpus {
    let mut plans = Vec::new();
    for &relation in ALL {
        let source = Input::Relation(relation);
        plans.push(plan(
            Operation::Select {
                from: source,
                comparisons: vec![],
                kept: vec![],
            },
            StepScope::PerFile,
        ));
        plans.push(plan(
            Operation::Project {
                from: source,
                columns: vec![ColumnIndex(0), ColumnIndex(0)],
                distinct: RepeatedRows::Kept,
            },
            StepScope::PerFile,
        ));
        plans.push(plan(
            Operation::Project {
                from: source,
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Dropped,
            },
            StepScope::PerFile,
        ));
        let mut dependent = plan(
            Operation::Select {
                from: source,
                comparisons: vec![],
                kept: vec![],
            },
            StepScope::PerFile,
        );
        dependent.steps.push(PlanStep {
            operation: Operation::Select {
                from: Input::Step(StepId(0)),
                comparisons: vec![],
                kept: vec![ColumnIndex(0)],
            },
            scope: StepScope::PerFile,
        });
        dependent.emit.step = StepId(1);
        plans.push(dependent);
        let mut continuation = plan(
            Operation::Select {
                from: source,
                comparisons: vec![],
                kept: vec![],
            },
            StepScope::PerFile,
        );
        if let Operation::Select { comparisons, .. } = &mut continuation.steps[0].operation {
            comparisons.push(Comparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(0)),
                right: Operand::Column(ColumnIndex(0)),
                origin: None,
            });
        }
        continuation.steps.push(PlanStep {
            operation: Operation::Project {
                from: Input::Step(StepId(0)),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Kept,
            },
            scope: StepScope::PerFile,
        });
        continuation.emit.step = StepId(1);
        plans.push(continuation);
        let invalid = ColumnIndex(
            u32::try_from(ryo_relations::domain::relations::service::columns(relation).len())
                .unwrap(),
        );
        plans.push(plan(
            Operation::Project {
                from: source,
                columns: vec![invalid],
                distinct: RepeatedRows::Kept,
            },
            StepScope::PerFile,
        ));
        plans.push(plan(
            Operation::Select {
                from: source,
                comparisons: vec![Comparison {
                    test: Test::Equal,
                    left: Operand::Column(invalid),
                    right: Operand::Column(ColumnIndex(0)),
                    origin: None,
                }],
                kept: vec![],
            },
            StepScope::PerFile,
        ));
        let mut project = plan(
            Operation::Select {
                from: source,
                comparisons: vec![],
                kept: vec![],
            },
            StepScope::CrossFile,
        );
        project.frontier.push(source);
        plans.push(project);
    }
    let bindings = vec![vec![]; plans.len()];
    let operators = RequestOperators::prepare_with_schemas(
        &plans,
        &bindings,
        &[],
        &ryo_relations::domain::relations::service::kinds,
        std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ),
    )
    .unwrap();
    let graph = crate::domain::file_program::service::compile_prepared(&plans, &operators);
    let program = crate::domain::executable_program::service::compile(
        &plans,
        graph,
        &FileSources,
        &operators,
    )
    .unwrap();
    let mut walk = super::super::PreparedWalk::compile(&plans, &bindings);
    walk.retain_program_consumers(&program, &bindings).unwrap();
    CompiledCorpus { program, walk }
}

// Text the plans carry in; it enters the engine here, as authored query text does.
// ignore: idiomatic.arch/idiomatic-rust/no-string-literals - authored query text enters the engine in this fixture
const CAPTURE_PATTERN: &str = "{parent}/{child}";
// ignore: idiomatic.arch/idiomatic-rust/no-string-literals - authored query text enters the engine in this fixture
const SETS_PARAMETER: &str = "sets";
// ignore: idiomatic.arch/idiomatic-rust/no-string-literals - authored query text enters the engine in this fixture
const ROWS_PARAMETER: &str = "rows";
// ignore: idiomatic.arch/idiomatic-rust/no-string-literals - authored query text enters the engine in this fixture
const ROW_KEY: &str = "key";
// ignore: idiomatic.arch/idiomatic-rust/no-string-literals - authored query text enters the engine in this fixture
const ROW_VALUE: &str = "value";
// ignore: idiomatic.arch/idiomatic-rust/no-string-literals - authored query text enters the engine in this fixture
const HOSTED_RELATION: &str = "hosted";
// ignore: idiomatic.arch/idiomatic-rust/no-string-literals - authored query text enters the engine in this fixture
const HOSTED_COLUMN: &str = "n";
// ignore: idiomatic.arch/idiomatic-rust/no-string-literals - authored query text enters the engine in this fixture
const TEXT_A: &str = "a";
// ignore: idiomatic.arch/idiomatic-rust/no-string-literals - authored query text enters the engine in this fixture
const TEXT_B: &str = "b";
// ignore: idiomatic.arch/idiomatic-rust/no-string-literals - authored query text enters the engine in this fixture
const TEXT_EMPTY: &str = "";

/// The function relation: id Node, name Symbol, file File, span Span.
pub(super) const FUNCTION: Relation = Relation::Function;
/// The syntax-child relation: parent Node, child Node, ordinal Number, field Symbol, file File.
pub(super) const EDGES: Relation = Relation::SyntaxChild;

/// Where an instruction's first row comes from, as the walk may route it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Route {
    Shared,
    Representation,
    Dependent,
    Observation,
    Row,
    Derived,
    Request,
    Continuation,
}

/// Whether a per-file relation head is independently eligible for the shared walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Eligibility {
    Eligible,
    Ineligible,
}

/// Where an instruction stands in its region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Place {
    Head(Eligibility),
    Later,
}

/// Whether the region's first input is a named observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FirstInput {
    Observation,
    Ordinary,
}

/// What an operation means for a source walk, chosen per kind with no wildcard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Meaning {
    /// Reads its input as selected or projected rows; the route follows the input.
    Selection,
    /// Computes new columns from each row.
    PerRow,
    /// Needs whole inputs: sets, joins, groups, ordering, graphs.
    WholeInput,
    /// Rows the request owns, not a relation.
    RequestOwned,
}

pub(super) fn meaning(kind: OperationKind) -> Meaning {
    match kind {
        OperationKind::Select | OperationKind::Project => Meaning::Selection,
        OperationKind::Extend
        | OperationKind::Capture
        | OperationKind::Rank
        | OperationKind::Replace
        | OperationKind::SpanBetween
        | OperationKind::SourceMatches
        | OperationKind::RenderSyntax
        | OperationKind::FitsRendered
        | OperationKind::SpanBounds => Meaning::PerRow,
        OperationKind::Join
        | OperationKind::Union
        | OperationKind::Intersection
        | OperationKind::Difference
        | OperationKind::Antijoin
        | OperationKind::GroupCount
        | OperationKind::GroupDigest
        | OperationKind::OrderedMatch
        | OperationKind::Words
        | OperationKind::Axis
        | OperationKind::Closure
        | OperationKind::OrderedDfs
        | OperationKind::Cycles => Meaning::WholeInput,
        OperationKind::Rows | OperationKind::Hosted => Meaning::RequestOwned,
    }
}

/// The route of the first instruction of a region.
/// `head` is the independent eligibility of a per-file relation head.
pub(super) fn head_route(
    operation: &Operation,
    first: FirstInput,
    head: Eligibility,
) -> Route {
    match meaning(kind_of(operation)) {
        Meaning::PerRow => Route::Row,
        Meaning::WholeInput => Route::Derived,
        Meaning::RequestOwned => Route::Request,
        Meaning::Selection => {
            let (Operation::Select { from, .. } | Operation::Project { from, .. }) = operation
            else {
                unreachable!("a selection meaning is Select or Project")
            };
            match from {
                Input::Relation(_) if head == Eligibility::Eligible => Route::Shared,
                Input::Relation(_) => Route::Representation,
                Input::BoundObservation(_) => Route::Observation,
                Input::Step(_) if first == FirstInput::Observation => Route::Observation,
                Input::Step(_) => Route::Dependent,
            }
        }
    }
}

/// The route of one instruction of a compiled program.
pub(super) fn route(
    program: &ExecutableProgram,
    region: &crate::domain::executable_program::types::Region,
    instruction: &crate::domain::executable_program::types::Instruction,
    place: Place,
) -> Route {
    if instruction.logical.0 >= program.logical_regions().len() {
        Route::Observation
    } else if let Place::Head(head) = place {
        head_route(
            &instruction.operation,
            if matches!(region.inputs.first(), Some(InputSource::Observation(_))) {
                FirstInput::Observation
            } else {
                FirstInput::Ordinary
            },
            head,
        )
    } else {
        Route::Continuation
    }
}

struct Sources;
impl ProducerPartitions for Sources {
    fn hosted(&self, _: RuleIndex, _: HostedNumber) -> Result<PartitionRequirement, OperatingError> {
        Ok(PartitionRequirement::RequestInvariant)
    }
    fn parameter(
        &self,
        _: RuleIndex,
        _: ParameterId,
    ) -> Result<PartitionRequirement, OperatingError> {
        Ok(PartitionRequirement::RequestInvariant)
    }
    fn relation(&self, _: Relation) -> PartitionRequirement {
        PartitionRequirement::CurrentFile
    }
    fn observation(
        &self,
        _: RuleIndex,
        _: ryo_values::domain::observations::types::ObservationBindingId,
    ) -> Result<PartitionRequirement, OperatingError> {
        Err(OperatingError::NotRead)
    }
}

fn from(relation: Relation) -> Input {
    Input::Relation(relation)
}
fn output() -> Vec<JoinColumn> {
    vec![
        JoinColumn { side: Side::Left, column: ColumnIndex(0) },
        JoinColumn { side: Side::Right, column: ColumnIndex(1) },
    ]
}
fn equal() -> Comparison {
    Comparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(0)),
        right: Operand::Column(ColumnIndex(0)),
        origin: None,
    }
}

/// One head operation for every variant, over `FUNCTION` and `EDGES`, in no particular order.
pub(super) fn operations() -> Vec<Operation> {
    let f = from(FUNCTION);
    let e = from(EDGES);
    vec![
        Operation::Select { from: f, comparisons: vec![], kept: vec![] },
        Operation::Project { from: f, columns: vec![ColumnIndex(0), ColumnIndex(0)], distinct: RepeatedRows::Kept },
        Operation::Extend {
            from: f,
            values: vec![Operand::Literal(QueryLiteral::Number(
                ryo_relations::domain::relations::types::Integer(4),
            ))],
        },
        Operation::Join {
            left: f,
            right: f,
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            output: output(),
            filter: vec![],
            view: vec![],
            reach: None,
        },
        Operation::Union { left: f, right: f },
        Operation::Intersection { left: f, right: f },
        Operation::Difference { left: f, right: f },
        Operation::Antijoin { left: f, right: f, left_on: vec![ColumnIndex(0)], right_on: vec![ColumnIndex(0)] },
        Operation::GroupCount { from: f, keys: vec![ColumnIndex(0)], distinct: None },
        Operation::GroupDigest { from: e, keys: vec![ColumnIndex(0)], over: ColumnIndex(2) },
        Operation::Capture { from: f, path: ColumnIndex(1), pattern: CapturePattern(CAPTURE_PATTERN.into()) },
        Operation::Rank { from: f, column: ColumnIndex(1), sets: ParameterId(0) },
        Operation::Replace {
            from: f,
            text: ColumnIndex(1),
            pattern: LiteralText(TEXT_A.into()),
            replacement: LiteralText(TEXT_B.into()),
        },
        Operation::OrderedMatch {
            left: e,
            right: e,
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            left_order: ColumnIndex(2),
            right_order: ColumnIndex(2),
            output: vec![
                JoinColumn { side: Side::Left, column: ColumnIndex(1) },
                JoinColumn { side: Side::Right, column: ColumnIndex(1) },
            ],
        },
        Operation::Words { from: f, text: ColumnIndex(1) },
        Operation::Axis {
            from: f,
            edges: e,
            domain: f,
            direction: Direction::Child,
            id: ColumnIndex(0),
            parent: ColumnIndex(0),
            child: ColumnIndex(1),
            domain_id: ColumnIndex(0),
            domain_filter: vec![],
            domain_view: vec![],
            output: vec![],
        },
        Operation::Closure { edges: e, keys: vec![], source: ColumnIndex(0), destination: ColumnIndex(1) },
        Operation::OrderedDfs {
            from: f,
            edges: e,
            root: ColumnIndex(0),
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
            order: ColumnIndex(2),
            edge: ColumnIndex(3),
            root_keys: vec![],
            edge_keys: vec![],
        },
        Operation::Cycles { edges: e, source: ColumnIndex(0), destination: ColumnIndex(1) },
        Operation::Rows { parameter: ParameterId(0), keys: vec![TableKey(ROW_KEY.into())] },
        Operation::Hosted { relation: HostedNumber(0) },
        Operation::SpanBetween { from: f, start: ColumnIndex(3), end: ColumnIndex(3) },
        Operation::SourceMatches {
            from: f,
            span: ColumnIndex(3),
            base_line: None,
            pattern: LiteralText(TEXT_A.into()),
            capture: None,
            capture_text: false,
            capture_line: false,
        },
        Operation::RenderSyntax {
            from: f,
            node: ColumnIndex(0),
            context: ryo_language::domain::semantics::types::RenderContext::PositionalArgument,
        },
        Operation::FitsRendered {
            from: f,
            texts: vec![ColumnIndex(1)],
            literals: vec![LiteralText(TEXT_EMPTY.into()), LiteralText(TEXT_EMPTY.into())],
            span: ColumnIndex(3),
            max_line: 88,
            tab_width: 4,
        },
        Operation::SpanBounds { from: f, span: ColumnIndex(3) },
    ]
}

/// A plan and the bindings of its rule.
struct Planned {
    plan: Plan,
    bindings: Vec<ParameterBinding>,
}

fn operation_plan(operation: Operation) -> Planned {
    let kind = kind_of(&operation);
    let mut plan = plan(operation, StepScope::PerFile);
    let mut bindings = vec![];
    if kind == OperationKind::Rank {
        plan.parameters.push(PlanParameter {
            name: ParameterName(SETS_PARAMETER.into()),
            kind: ParameterType::TextSets,
        });
        bindings.push(ParameterBinding::TextSets(vec![vec![LiteralText(TEXT_A.into())]]));
    }
    if kind == OperationKind::Rows {
        plan.parameters.push(PlanParameter {
            name: ParameterName(ROWS_PARAMETER.into()),
            kind: ParameterType::Tables,
        });
        bindings.push(ParameterBinding::Tables(vec![BindingTable(vec![BindingCell {
            key: LiteralText(ROW_KEY.into()),
            value: LiteralText(ROW_VALUE.into()),
        }])]));
    }
    Planned { plan, bindings }
}

/// Plans beside the variants: relation heads that share, one that does not, a Step input,
/// a fused continuation and a project-stage head.
fn shapes() -> Vec<Plan> {
    let f = from(FUNCTION);
    let select = |comparisons| Operation::Select { from: f, comparisons, kept: vec![] };
    let mut plans = vec![
        plan(select(vec![]), StepScope::PerFile),
        plan(select(vec![equal()]), StepScope::PerFile),
        plan(
            Operation::Project { from: f, columns: vec![ColumnIndex(0)], distinct: RepeatedRows::Dropped },
            StepScope::PerFile,
        ),
    ];
    let mut dependent = plan(select(vec![]), StepScope::PerFile);
    dependent.steps.push(ryo_values::domain::plans::types::PlanStep {
        operation: Operation::Select {
            from: Input::Step(StepId(0)),
            comparisons: vec![],
            kept: vec![ColumnIndex(0)],
        },
        scope: StepScope::PerFile,
    });
    dependent.emit.step = StepId(1);
    plans.push(dependent);
    let mut continuation = plan(select(vec![equal()]), StepScope::PerFile);
    continuation.steps.push(ryo_values::domain::plans::types::PlanStep {
        operation: Operation::Project {
            from: Input::Step(StepId(0)),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Kept,
        },
        scope: StepScope::PerFile,
    });
    continuation.emit.step = StepId(1);
    plans.push(continuation);
    let mut project = plan(select(vec![]), StepScope::CrossFile);
    project.frontier.push(f);
    plans.push(project);
    plans
}

pub(super) struct Corpus {
    pub program: ExecutableProgram,
    pub walk: super::super::PreparedWalk,
}

/// Every variant and shape through the normal operator, graph, program and walk compilers.
pub(super) fn compiled_operations() -> Corpus {
    let mut plans = Vec::new();
    let mut bindings = Vec::new();
    for operation in operations() {
        let planned = operation_plan(operation);
        plans.push(planned.plan);
        bindings.push(planned.bindings);
    }
    for shape in shapes() {
        plans.push(shape);
        bindings.push(vec![]);
    }
    let hosted = vec![HostRelation {
        name: HostName(HOSTED_RELATION.into()),
        columns: vec![HostColumn { name: HostColumnName(HOSTED_COLUMN.into()), kind: HostKind::Number }],
        rows: vec![],
    }];
    let operators = RequestOperators::prepare_with_schemas(
        &plans,
        &bindings,
        &hosted,
        &ryo_relations::domain::relations::service::kinds,
        std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ),
    )
    .unwrap();
    let graph = crate::domain::file_program::service::compile_prepared(&plans, &operators);
    let program =
        crate::domain::executable_program::service::compile(&plans, graph, &Sources, &operators)
            .unwrap();
    let mut walk = super::super::PreparedWalk::compile(&plans, &bindings);
    walk.retain_program_consumers(&program, &bindings).unwrap();
    Corpus { program, walk }
}
