//! Actual common-engine routes must reach the same injected Join capability.
use super::*;
use crate::application::file_dag::execution::CommonOperatorEngine;
use crate::domain::operating::types::Memo;
use crate::domain::table_model::types::{SelectedRows, StoredSelection};
use crate::outbound::operators::interface::OperatorsPort;
use ryo_operators::inbound::use_cases::operators::interface::{
    OperatorAnswer, OperatorCall, PortableOperatorFault,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
const REFUSAL: u32 = 60001;
fn feed_owner() -> std::rc::Rc<crate::application::file_dag::relations::RelationOwner> {
    std::rc::Rc::new(crate::application::file_dag::relations::RelationOwner::new(
        true,
        (std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    ))
}
fn stream_with_port(
    port: &dyn OperatorsPort,
    contract: &crate::domain::operator::contract::types::OperatorContract,
    feed: &mut ControlledFeed<'_>,
    context: &mut OperatorContext<'_>,
    room: &mut Room,
    emit: &mut dyn crate::domain::operating::types::Emit,
) -> Result<(), OperatingError> {
    let owner = std::rc::Rc::clone(&feed.owner);
    CommonOperatorEngine::new(port, owner.derived.semantic_port())
        .stream(&owner, contract, feed, context, room, emit)
}
#[derive(Debug)]
pub(in crate::application::file_dag::acceptance) struct Refuses {
    calls: AtomicUsize,
    positions: AtomicUsize,
    columns: AtomicUsize,
    refuse: bool,
    only_columns: bool,
}
impl Default for Refuses {
    fn default() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            positions: AtomicUsize::new(0),
            columns: AtomicUsize::new(0),
            refuse: true,
            only_columns: false,
        }
    }
}
impl Refuses {
    pub(in crate::application::file_dag::acceptance) fn forwarding() -> Self {
        Self {
            refuse: false,
            ..Self::default()
        }
    }
    pub(in crate::application::file_dag::acceptance) fn column_refusal() -> Self {
        Self {
            only_columns: true,
            ..Self::default()
        }
    }
    pub(in crate::application::file_dag::acceptance) fn column_calls(&self) -> usize {
        self.columns.load(Ordering::Relaxed)
    }
    pub(in crate::application::file_dag::acceptance) fn all_calls(&self) -> usize {
        self.calls.load(Ordering::Relaxed)
    }
}
impl OperatorsPort for Refuses {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        if let OperatorCall::EqualityJoin { input } = call {
            let columnar = match &input {
                ryo_operators::inbound::use_cases::operators::interface::JoinInput::Values {
                    left,
                    ..
                } => match left {
                    ryo_operators::inbound::use_cases::operators::interface::JoinRow::Values(_) => {
                        false
                    }
                    ryo_operators::inbound::use_cases::operators::interface::JoinRow::Columns(
                        _,
                    ) => {
                        self.columns.fetch_add(1, Ordering::Relaxed);
                        true
                    }
                },
                ryo_operators::inbound::use_cases::operators::interface::JoinInput::Positions {
                    ..
                } => {
                    self.positions.fetch_add(1, Ordering::Relaxed);
                    false
                }
            };
            self.calls.fetch_add(1, Ordering::Relaxed);
            if self.refuse && (!self.only_columns || columnar) {
                return Err(PortableOperatorFault::NoSuchColumn { index: REFUSAL });
            }
            (std::sync::Arc::new(
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
            )
                as std::sync::Arc<dyn crate::outbound::operators::interface::OperatorsPort>)
                .operators(OperatorCall::EqualityJoin { input })
        } else {
            (std::sync::Arc::new(
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
            )
                as std::sync::Arc<dyn crate::outbound::operators::interface::OperatorsPort>)
                .operators(call)
        }
    }
}
fn refused(result: Result<(), OperatingError>) {
    assert!(
        matches!(
            &result,
            Err(OperatingError::Table(
                ryo_values::domain::rows::errors::TableModelError::NoSuchColumn { index: REFUSAL }
            ))
        ),
        "{result:?}"
    );
}
fn context<'a>(
    symbols: &'a mut Symbols,
    scratch: &'a mut OperatorScratch,
    memo: &'a mut Vec<crate::domain::operating::types::Memo>,
) -> OperatorContext<'a> {
    OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: symbols.into(),
        source: SourceView(b"abcd"),
        paths: &[],
        arenas: &[],
        memo,
        scratch,
    }
}
fn collected(operation: &Operation, inputs: &[Rows<'_>], symbols: &mut Symbols, limit: usize) {
    let fixture = ResourceFixture::new(&[Relation::Function], &[], limit).expect("actual owner");
    let port = Refuses::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = vec![];
    let mut ctx = context(symbols, &mut scratch, &mut memo);
    let mut output = Table::default();
    refused(fixture.collect_with(&port, operation, inputs, &mut ctx, &mut output));
    assert_eq!(port.calls.load(Ordering::Relaxed), 1);
    assert!(fixture.no_invocation_index_handles());
    // Refusal cannot poison the same owner's following real evaluation, including zero optional budget.
    fixture
        .collect(operation, inputs, &mut ctx, &mut output)
        .expect("retry through real capability");
    assert!(output.count.0 > 0);
    assert!(fixture.no_invocation_index_handles());
}
#[test]
fn collected_computed_stored_selected_and_generic_routes_use_injected_join() {
    let left = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(7))]]);
    let right = table_of(
        NUMBER_PAIR_KINDS,
        &[
            &[Cell::Number(Integer(7)), Cell::Number(Integer(1))],
            &[Cell::Number(Integer(7)), Cell::Number(Integer(2))],
        ],
    );
    let mut words = ryo_relations::domain::relations::types::Words::default();
    ryo_values::domain::table_words::service::encode_table(&right, &[], &mut words);
    let (compact, names) = ryo_values::domain::table_words::service::admit_compact(
        Arc::new(ryo_values::domain::stored_facts::types::KeptWords(
            ryo_values::domain::table_words::service::pack(
                ryo_values::domain::table_words::types::WordSlice(&words.0),
            ),
        )),
        &mut Vec::new(),
        ryo_values::domain::table_words::types::DecodeMode::Complete,
    )
    .unwrap();
    assert!(names.is_empty());
    assert!(ryo_values::domain::table_words::service::compact_rectangular(&compact));
    let columns = [fact_column(&[7, 7]), fact_column(&[1, 2])];
    let stored = ColumnsView {
        kinds: NUMBER_PAIR_KINDS,
        columns: &columns,
    };
    let selected = Rows::Selected(SelectedRows {
        source: stored,
        selection: Arc::new(StoredSelection {
            relation: Relation::Function,
            rows: Arc::new(vec![1, 0, 1]),
            columns: vec![ColumnIndex(0), ColumnIndex(1)],
            kinds: NUMBER_PAIR_KINDS.to_vec(),
            identity: Arc::new(Default::default()),
        }),
    });
    let ordinary = plan(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    use crate::domain::table_model::collections::representations::types::RepresentationKind;
    for representation in RepresentationKind::ALL {
        let rows = match representation {
            RepresentationKind::Stored => Rows::Stored(stored),
            RepresentationKind::Selected => selected.clone(),
            RepresentationKind::Computed => Rows::Computed(&right),
            RepresentationKind::Compact => Rows::Compact(&compact),
            RepresentationKind::Tree => {
                let (elements, mut symbols) = wide_tree(1, true);
                let nodes = table_of(&[CellKind::Node], &[&[node(1)]]);
                collected(
                    &plan(&[0], &[0], &[(Side::Right, 0)], vec![], &[]),
                    &[
                        Rows::Computed(&nodes),
                        tree_rows(&elements, TreeForm::Elements),
                    ],
                    &mut symbols,
                    0,
                );
                continue;
            }
        };
        for limit in [0, crate::application::file_dag::derived::indexes::LIMIT] {
            collected(
                &ordinary,
                &[Rows::Computed(&left), rows.clone()],
                &mut Symbols::default(),
                limit,
            );
        }
    }
    // A tree LHS declines the collected physical Join shortcut, so the
    // mandatory row evaluator must still reach the same injected boundary.
    let (elements, mut symbols) = wide_tree(1, true);
    let right = table_of(&[CellKind::Node], &[&[node(1)]]);
    let generic = plan(&[0], &[0], &[(Side::Right, 0)], vec![], &[]);
    collected(
        &generic,
        &[
            tree_rows(&elements, TreeForm::Elements),
            Rows::Computed(&right),
        ],
        &mut symbols,
        0,
    );
}
#[test]
fn collected_sparse_left_and_native_ready_routes_use_injected_join() {
    if diagnostic_case_in_child(
        "application::file_dag::acceptance::operator_collections_operators_join_item_test::capability_test::collected_sparse_left_and_native_ready_routes_use_injected_join",
        "RYO_JOIN_CAPABILITY_SPARSE_CHILD",
    ) {
        return;
    }
    let diagnostics = JoinDiagnostics::default();
    let capture = diagnostics.clone();
    let dispatch = tracing::Dispatch::new(diagnostics);
    let _guard = tracing::dispatcher::set_default(&dispatch);
    let columns = sparse_function_columns(256);
    let left = Rows::Stored(ColumnsView {
        kinds: ryo_relations::domain::relations::service::kinds(Relation::Function),
        columns: &columns,
    });
    let right = table_of(&[CellKind::Node], &[&[node(1)]]);
    let sparse = plan(&[0], &[0], &[(Side::Right, 0)], vec![], &[]);
    collected(
        &sparse,
        &[left, Rows::Computed(&right)],
        &mut Symbols::default(),
        crate::application::file_dag::derived::indexes::LIMIT,
    );
    assert!(
        capture
            .events
            .lock()
            .expect("real sparse events")
            .iter()
            .any(|event| event.get("path").map(String::as_str) == Some("sparse-left-key")),
        "sparse physical route was not executed"
    );
    let (elements, mut symbols) = wide_tree(1, true);
    let left = table_of(&[CellKind::Node], &[&[node(1)]]);
    collected(
        &sparse,
        &[
            Rows::Computed(&left),
            tree_rows(&elements, TreeForm::Elements),
        ],
        &mut symbols,
        0,
    );
}
fn prepared(operation: &Operation, rows: &[Rows<'_>]) -> OperatorContract {
    let schemas = rows
        .iter()
        .map(|rows| InputShape {
            kinds: kinds_of(rows),
            tree: if let Rows::Tree(tree) = rows {
                Some(tree.form)
            } else {
                None
            },
        })
        .collect::<Vec<_>>();
    crate::application::file_dag::derived::prepare::operator(
        crate::domain::operator::service::registered(
            ryo_values::domain::plans::types::OperationKind::Join,
        ),
        operation,
        &schemas,
        &[],
    )
    .expect("prepared join")
}
#[test]
fn streamed_selected_and_native_rows_use_injected_join_and_empty_left_does_not_call_it() {
    let columns = [fact_column(&[7, 7]), fact_column(&[1, 2])];
    let selected = Rows::Selected(SelectedRows {
        source: ColumnsView {
            kinds: NUMBER_PAIR_KINDS,
            columns: &columns,
        },
        selection: Arc::new(StoredSelection {
            relation: Relation::Function,
            rows: Arc::new(vec![1, 0, 1]),
            columns: vec![ColumnIndex(0), ColumnIndex(1)],
            kinds: NUMBER_PAIR_KINDS.to_vec(),
            identity: Arc::new(Default::default()),
        }),
    });
    let left = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(7))]]);
    stream(
        &plan(&[0], &[0], &[(Side::Right, 1)], vec![], &[]),
        &left,
        selected,
        &mut Symbols::default(),
    );
    let (elements, mut symbols) = wide_tree(1, true);
    let nodes = table_of(&[CellKind::Node], &[&[node(1)]]);
    stream(
        &plan(&[0], &[0], &[(Side::Right, 0)], vec![], &[]),
        &nodes,
        tree_rows(&elements, TreeForm::Elements),
        &mut symbols,
    );
}
fn stream(operation: &Operation, left: &Table, right: Rows<'_>, symbols: &mut Symbols) {
    let contract = prepared(operation, &[Rows::Computed(left), right.clone()]);
    let port = Refuses::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = vec![];
    let mut ctx = context(symbols, &mut scratch, &mut memo);
    let mut feed = ControlledFeed {
        owner: feed_owner(),
        left,
        right: right.clone(),
        right_holds: 0,
        left_fails: false,
        right_fails: false,
    };
    let mut output = CollectedRows::default();
    refused(stream_with_port(
        &port,
        &contract,
        &mut feed,
        &mut ctx,
        &mut Room::default(),
        &mut output,
    ));
    assert_eq!(port.calls.load(Ordering::Relaxed), 1);
    assert!(output.rows.is_empty());
    let empty = table_of(&left.kinds, &[]);
    let mut feed = ControlledFeed {
        owner: feed_owner(),
        left: &empty,
        right,
        right_holds: 0,
        left_fails: false,
        right_fails: false,
    };
    stream_with_port(
        &port,
        &contract,
        &mut feed,
        &mut ctx,
        &mut Room::default(),
        &mut output,
    )
    .expect("empty left");
    assert_eq!(port.calls.load(Ordering::Relaxed), 1);
    assert!(output.rows.is_empty());
}
#[derive(Debug)]
struct KeepsCalling;
impl OperatorsPort for KeepsCalling {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        if let OperatorCall::EqualityJoin {
            input:
                ryo_operators::inbound::use_cases::operators::interface::JoinInput::Values {
                    driver,
                    ..
                },
        } = call
        {
            assert!(driver.emit(&[Cell::Number(Integer(1))]).is_err());
            assert!(driver.emit(&[Cell::Number(Integer(2))]).is_err());
            assert!(driver.next(&[]).is_err());
            // The original delivery failure must win even over dishonest success.
            Ok(OperatorAnswer::Written)
        } else {
            (std::sync::Arc::new(
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
            )
                as std::sync::Arc<dyn crate::outbound::operators::interface::OperatorsPort>)
                .operators(call)
        }
    }
}
#[test]
fn first_downstream_failure_remains_sticky_even_if_injected_kernel_returns_success() {
    let left = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(7))]]);
    let right = table_of(
        NUMBER_PAIR_KINDS,
        &[&[Cell::Number(Integer(7)), Cell::Number(Integer(1))]],
    );
    let operation = plan(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    let contract = prepared(&operation, &[Rows::Computed(&left), Rows::Computed(&right)]);
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = vec![];
    let mut ctx = context(&mut symbols, &mut scratch, &mut memo);
    let mut feed = ControlledFeed {
        owner: feed_owner(),
        left: &left,
        right: Rows::Computed(&right),
        right_holds: 0,
        left_fails: false,
        right_fails: false,
    };
    let mut output = CollectedRows {
        rows: vec![],
        fails: true,
    };
    assert!(matches!(
        stream_with_port(
            &KeepsCalling,
            &contract,
            &mut feed,
            &mut ctx,
            &mut Room::default(),
            &mut output
        ),
        Err(OperatingError::NotRun)
    ));
    assert!(output.rows.is_empty());
}

#[test]
fn upstream_failure_and_empty_left_do_not_probe_rhs_or_enter_join_capability() {
    let left = table_of(&[CellKind::Number], &[]);
    let columns = [fact_column(&[7]), fact_column(&[1])];
    let right = Rows::Stored(ColumnsView {
        kinds: NUMBER_PAIR_KINDS,
        columns: &columns,
    });
    let operation = plan(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    let contract = prepared(&operation, &[Rows::Computed(&left), right.clone()]);
    let port = Refuses::default();
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = vec![];
    let mut ctx = context(&mut symbols, &mut scratch, &mut memo);
    let mut room = Room::default();
    let mut output = CollectedRows::default();
    for fails in [false, true] {
        let mut feed = ControlledFeed {
            owner: feed_owner(),
            left: &left,
            right: right.clone(),
            right_holds: 0,
            left_fails: fails,
            right_fails: false,
        };
        let result = stream_with_port(
            &port,
            &contract,
            &mut feed,
            &mut ctx,
            &mut room,
            &mut output,
        );
        if fails {
            assert!(matches!(result, Err(OperatingError::NotRun)));
        } else {
            result.expect("empty lhs");
        }
        assert_eq!(port.calls.load(Ordering::Relaxed), 0);
        // This transport intentionally holds the at-hand RHS before streaming the
        // LHS; an early failure/empty stream must not build or probe its index.
        assert_eq!(feed.right_holds, 1);
        assert_eq!(room.scratch.observation.index_rows, 0);
        assert!(output.rows.is_empty());
    }
}
#[test]
fn original_engine_error_wins_over_every_inconsistent_kernel_terminal() {
    use crate::domain::operator::contract::types::join_result;
    for answer in [
        Ok(OperatorAnswer::Written),
        Ok(OperatorAnswer::SpanBounds([Cell::Number(Integer(1)); 2])),
        Err(PortableOperatorFault::InvalidSpan),
    ] {
        assert!(matches!(
            join_result(answer, Some(OperatingError::NotRead)),
            Err(OperatingError::NotRead)
        ));
    }
    assert!(matches!(
        join_result(Err(PortableOperatorFault::Interrupted), None),
        Err(OperatingError::WrongOperation)
    ));
    assert!(matches!(
        join_result(Err(PortableOperatorFault::InvalidSpan), None),
        Err(OperatingError::InvalidSpan)
    ));
}

#[test]
fn lowered_pipeline_join_cannot_bypass_the_same_refusing_capability() {
    use crate::domain::operator::types::TreeSteps;
    for strategy in [TreeSteps::Lowered, TreeSteps::AsOperators] {
        for right in [
            PlanInput::Relation(Relation::SyntaxTree),
            PlanInput::Relation(Relation::SyntaxChild),
            PlanInput::Step(ryo_values::domain::plan_parts::types::StepId(1)),
        ] {
            match strategy {
                TreeSteps::Lowered => native_pipeline_case(TreeSteps::Lowered, right),
                TreeSteps::AsOperators => native_pipeline_case(TreeSteps::AsOperators, right),
            }
        }
    }
}
fn native_pipeline_case(strategy: crate::domain::operator::types::TreeSteps, right: PlanInput) {
    use crate::domain::operator::{
        service::pool_for,
        types::{PreparedPool, TreeSteps},
    };
    use ryo_values::domain::plan_parts::types::StepId;
    use ryo_values::domain::plans::types::{Emit, Plan, PlanStep, StepScope};
    let (mut elements, mut symbols) = wide_tree(1, true);
    for (bits, name) in [(1, "file"), (2, "word")] {
        name_kind(
            &mut completion(&mut elements, &mut symbols, ByteSlice(b"abcd"), FileId(0)),
            KindBits(bits),
            NameText(name),
        );
    }
    index_kinds(&mut elements);
    let select = Operation::Select {
        from: PlanInput::Relation(Relation::SyntaxTree),
        comparisons: vec![],
        kept: vec![ColumnIndex(0)],
    };
    let mut steps = vec![PlanStep {
        operation: select.clone(),
        scope: StepScope::PerFile,
    }];
    if matches!(right, PlanInput::Step(_)) {
        steps.push(PlanStep {
            operation: select,
            scope: StepScope::PerFile,
        });
    }
    let root = StepId(u32::try_from(steps.len()).expect("bounded fixture steps"));
    let join = Operation::Join {
        left: PlanInput::Step(StepId(0)),
        right,
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(
            if right == PlanInput::Relation(Relation::SyntaxChild) {
                1
            } else {
                0
            },
        )],
        output: vec![JoinColumn {
            side: Side::Left,
            column: ColumnIndex(0),
        }],
        filter: vec![],
        view: vec![],
        reach: None,
    };
    steps.push(PlanStep {
        operation: join,
        scope: StepScope::PerFile,
    });
    let plan = Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: vec![],
        steps,
        emit: Emit {
            step: root,
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
        comparison_origins: vec![],
    };
    let port = Refuses::default();
    let engine = CommonOperatorEngine::new(
        &port,
        (std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    );
    let request = crate::domain::operator::contract::types::RequestOperators::prepare(
        std::slice::from_ref(&plan),
        &[vec![]],
        &[],
        engine.derived.clone(),
    )
    .expect("request preparation");
    let mut prepared = PreparedPool {
        unlowered: strategy,
        ..PreparedPool::default()
    };
    crate::application::file_dag::derived::selection::prepare_request(&mut prepared, &request);
    let scope = crate::application::file_dag::derived::owner::resource_fixture::prepared_scope(
        std::slice::from_ref(&plan),
        &request,
        &prepared,
        &[Relation::SyntaxTree, Relation::SyntaxChild],
        &[],
        &engine,
    )
    .expect("required execution scope");
    let joined = *scope.scope.graph().program_nodes[0][root.0 as usize]
        .as_ref()
        .expect("compiled join");
    assert_eq!(
        scope.scope.graph().nodes[joined.0].input_supply[0],
        crate::domain::operating::types::Supply::AtHand,
        "the prepared join needs the left publication identity"
    );
    let plans = [plan.clone()];
    let bindings = [Vec::new()];
    let mut merged = crate::domain::shared_walk::types::MergedWalk::default();
    let mut classes = crate::domain::shared_walk::types::SharedClasses::default();
    let mut native_scratch = OperatorScratch::default();
    let mask = crate::domain::placement::types::Mask::default();
    crate::application::file_dag::types::FileDagExecution::prepare_native(
        &scope.execution,
        &mut merged,
        crate::domain::shared_walk::types::Walking {
            plans: &plans,
            bindings: &bindings,
            paths: &[],
            mask: &mask,
            file: FileId(0),
            elements: &elements,
            symbols: &mut symbols,
            source: SourceView(b"abcd"),
            scratch: &mut native_scratch,
            room: &mut classes,
            steps: strategy,
        },
        crate::domain::column_kernel::types::KernelStrategy::Existing,
        crate::domain::lowering::types::Counting::DoesNotCount,
        scope.scope.executor(),
    )
    .expect("native preparation uses the same required scope");
    let mut pool = pool_for(std::slice::from_ref(&plan));
    let mut scratch = OperatorScratch::default();
    let mut memo = vec![];
    let mut ctx = context(&mut symbols, &mut scratch, &mut memo);
    let read = |_| None;
    let _previous = crate::domain::table_model::service::take_read_row_counts();
    let result = crate::application::file_dag::pipeline::service::evaluate_file_with_executor(
        &plan,
        &read,
        Some(TreeRows {
            elements: &elements,
            file: FileId(0),
            form: TreeForm::Elements,
        }),
        &mut ctx,
        &mut pool,
        &mut prepared,
        0,
        &scope.execution,
        scope.scope.executor(),
    );
    let state = scope.execution.borrow();
    assert!(
        matches!(
            &result,
            Err(OperatingError::Table(
                ryo_values::domain::rows::errors::TableModelError::NoSuchColumn { index: REFUSAL }
            ))
        ),
        "strategy={strategy:?}, right={right:?}, root={:?}, result={result:?}, failed={:?}, visiting={:?}, executions={:?}, supplies={:?}",
        scope.scope.graph().program_nodes[0][root.0 as usize],
        state.failed,
        state.visiting,
        state.executions,
        scope
            .scope
            .graph()
            .nodes
            .iter()
            .map(|node| &node.input_supply)
            .collect::<Vec<_>>(),
    );
    drop(state);
    assert_eq!(port.calls.load(Ordering::Relaxed), 1);
    let reads = crate::domain::table_model::service::take_read_row_counts();
    match strategy {
        TreeSteps::Lowered => {
            assert_eq!(
                port.positions.load(Ordering::Relaxed),
                1,
                "must use deferred positions, not value fallback"
            );
            assert!(
                scope.scope.graph().program_nodes[0][root.0 as usize]
                    .as_ref()
                    .ok()
                    .and_then(|id| scope.scope.graph().nodes.get(id.0))
                    .is_some_and(|node| node.regions.iter().any(|region| {
                        region.layout.lowering.shapes[root.0 as usize].is_some()
                    }))
            );
            assert_eq!(
                reads,
                (0, 0),
                "lowered refusal must not materialize typed rows"
            );
        }
        TreeSteps::AsOperators => assert_eq!(
            port.positions.load(Ordering::Relaxed),
            0,
            "ordinary native strategy must use the value capability"
        ),
    }
}

struct NativeOrderedFeed<'a> {
    owner: std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
    left: &'a Table,
    right: &'a Table,
    left_fails: bool,
    right_fails: bool,
    left_holds: usize,
    right_holds: usize,
}

impl<'f> Feed<'f> for NativeOrderedFeed<'f> {
    fn relation_owner(
        &self,
    ) -> std::rc::Rc<crate::application::file_dag::relations::RelationOwner> {
        std::rc::Rc::clone(&self.owner)
    }
    fn origin(
        &self,
        which: InputPlace,
    ) -> Result<crate::application::file_dag::feed::InputOrigin, OperatingError> {
        crate::application::file_dag::testing::external_origin(which, 2)
    }
    fn publication(
        &self,
        which: InputPlace,
    ) -> Result<Option<crate::application::file_dag::relations::PublishedInput>, OperatingError>
    {
        crate::application::file_dag::testing::no_publication(which, 2)
    }
    fn kinds(&self, which: InputPlace) -> Result<&[CellKind], OperatingError> {
        match which.0 {
            0 => Ok(&self.left.kinds),
            1 => Ok(&self.right.kinds),
            _ => Err(OperatingError::MissingInput {
                position: which.0 + 1,
            }),
        }
    }
    fn streams(&self, which: InputPlace) -> Result<Supply, OperatingError> {
        crate::application::file_dag::testing::supplied(which, 2, Supply::AtHand)
    }
    fn native_join_right_first(&self) -> bool {
        true
    }
    fn stream(
        &mut self,
        _: InputPlace,
        _: &mut OperatorContext<'_>,
        _: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        Err(OperatingError::WrongOperation)
    }
    fn found(&mut self, _: InputPlace, _: RowCount) {}
    fn hold<'t>(
        &mut self,
        which: InputPlace,
        _: &mut OperatorContext<'_>,
        _: &'t mut Table,
    ) -> Result<Rows<'t>, OperatingError>
    where
        'f: 't,
    {
        match which.0 {
            0 => {
                self.left_holds += 1;
                if self.left_fails {
                    Err(OperatingError::NotRun)
                } else {
                    Ok(Rows::Computed(self.left))
                }
            }
            1 => {
                self.right_holds += 1;
                if self.right_fails {
                    Err(OperatingError::NotRead)
                } else {
                    Ok(Rows::Computed(self.right))
                }
            }
            _ => Err(OperatingError::MissingInput {
                position: which.0 + 1,
            }),
        }
    }
}

#[test]
fn native_provenance_join_keeps_right_first_refusal_and_empty_left_order() {
    let empty = table_of(&[CellKind::Number], &[]);
    let nonempty = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(7))]]);
    let operation = plan(&[0], &[0], &[(Side::Left, 0)], vec![], &[]);
    let contract = prepared(
        &operation,
        &[Rows::Computed(&empty), Rows::Computed(&nonempty)],
    );
    let port = (std::sync::Arc::new(
        ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
    ) as std::sync::Arc<dyn crate::outbound::operators::interface::OperatorsPort>);
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = vec![];
    let mut context = context(&mut symbols, &mut scratch, &mut memo);
    for (left_fails, right_fails, expected, expected_left_holds) in [
        (false, true, Err(OperatingError::NotRead), 0),
        (true, true, Err(OperatingError::NotRead), 0),
        (true, false, Err(OperatingError::NotRun), 1),
        (false, false, Ok(()), 1),
    ] {
        let owner = feed_owner();
        let engine = CommonOperatorEngine::new(port, owner.derived.semantic_port());
        let mut feed = NativeOrderedFeed {
            owner: std::rc::Rc::clone(&owner),
            left: &empty,
            right: &nonempty,
            left_fails,
            right_fails,
            left_holds: 0,
            right_holds: 0,
        };
        let mut room = Room::default();
        let mut output = CollectedRows::default();
        let result = engine.stream(
            &owner,
            &contract,
            &mut feed,
            &mut context,
            &mut room,
            &mut output,
        );
        assert_eq!(result, expected);
        assert_eq!(
            feed.right_holds, 1,
            "the formerly streamed Join held the right first"
        );
        assert_eq!(feed.left_holds, expected_left_holds);
        assert!(output.rows.is_empty(), "empty left cannot emit a Join row");
    }
}

struct NestedOutput {
    rows: Vec<Vec<Cell>>,
    fail_at: Option<usize>,
    entries: usize,
}
impl Emit for NestedOutput {
    fn row(
        &mut self,
        context: &mut OperatorContext<'_>,
        row: &[Cell],
    ) -> Result<(), OperatingError> {
        self.entries += 1;
        // This state belongs to this invocation's downstream consumer. Nested
        // preparation may not replace or mutate it while this callback resumes.
        context.memo.clear();
        context.memo.push(Memo::Holds);
        context.scratch.row = vec![Cell::Number(Integer(987))];
        let left = table_of(&[CellKind::Number], &[row]);
        let right = table_of(NUMBER_PAIR_KINDS, &[&[row[0], Cell::Number(Integer(42))]]);
        let operation = plan(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
        let contract = prepared(&operation, &[Rows::Computed(&left), Rows::Computed(&right)]);
        let mut feed = ControlledFeed {
            owner: feed_owner(),
            left: &left,
            right: Rows::Computed(&right),
            right_holds: 0,
            left_fails: false,
            right_fails: false,
        };
        let mut nested = CollectedRows::default();
        stream_with_port(
            (std::sync::Arc::new(
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
            )
                as std::sync::Arc<dyn crate::outbound::operators::interface::OperatorsPort>),
            &contract,
            &mut feed,
            context,
            &mut Room::default(),
            &mut nested,
        )?;
        assert_eq!(nested.rows, vec![vec![Cell::Number(Integer(42))]]);
        assert_eq!(*context.memo, vec![Memo::Holds]);
        assert_eq!(context.scratch.row, vec![Cell::Number(Integer(987))]);
        if self.fail_at == Some(self.rows.len()) {
            return Err(OperatingError::NotRead);
        }
        self.rows.push(row.to_vec());
        Ok(())
    }
}
#[test]
fn shared_output_scope_preserves_nested_state_error_prefix_and_next_invocation_resume() {
    let left = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(7))]]);
    let columns = [fact_column(&[7, 7, 7]), fact_column(&[1, 2, 3])];
    let right = Rows::Stored(ColumnsView {
        kinds: NUMBER_PAIR_KINDS,
        columns: &columns,
    });
    let operation = plan(&[0], &[0], &[(Side::Right, 0)], vec![], &[]);
    let contract = prepared(&operation, &[Rows::Computed(&left), right.clone()]);
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    scratch.row = vec![Cell::Number(Integer(654))];
    let mut memo = vec![Memo::Fails];
    let mut ctx = context(&mut symbols, &mut scratch, &mut memo);
    let mut room = Room::default();
    let mut output = NestedOutput {
        rows: vec![],
        fail_at: Some(1),
        entries: 0,
    };
    let mut feed = ControlledFeed {
        owner: feed_owner(),
        left: &left,
        right: right.clone(),
        right_holds: 0,
        left_fails: false,
        right_fails: false,
    };
    assert!(matches!(
        stream_with_port(
            (std::sync::Arc::new(
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new()
            )
                as std::sync::Arc<dyn crate::outbound::operators::interface::OperatorsPort>),
            &contract,
            &mut feed,
            &mut ctx,
            &mut room,
            &mut output
        ),
        Err(OperatingError::NotRead)
    ));
    assert_eq!(output.rows, vec![vec![Cell::Number(Integer(7))]]);
    assert_eq!(output.entries, 2);
    assert_eq!(*ctx.memo, vec![Memo::Fails]);
    assert_eq!(ctx.scratch.row, vec![Cell::Number(Integer(654))]);
    output.rows.clear();
    output.fail_at = None;
    output.entries = 0;
    let mut feed = ControlledFeed {
        owner: feed_owner(),
        left: &left,
        right,
        right_holds: 0,
        left_fails: false,
        right_fails: false,
    };
    stream_with_port(
        (std::sync::Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::operators::interface::OperatorsPort>),
        &contract,
        &mut feed,
        &mut ctx,
        &mut room,
        &mut output,
    )
    .expect("next invocation resumes");
    assert_eq!(output.rows, vec![vec![Cell::Number(Integer(7))]; 3]);
    assert_eq!(output.entries, 3);
    assert_eq!(*ctx.memo, vec![Memo::Fails]);
    assert_eq!(ctx.scratch.row, vec![Cell::Number(Integer(654))]);
}
