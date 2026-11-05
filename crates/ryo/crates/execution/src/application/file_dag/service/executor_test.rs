//! A DAG demand must use the same injected primitive executor as pipeline steps.
use super::*;
use crate::application::file_dag::executor::PrimitiveExecutor;
use crate::domain::operating::types::Room;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{ColumnsView, FactColumn, FileId, Integer},
};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, JoinColumn, RepeatedRows, Side, StepId},
    plans::types::{Emit as PlanEmit, Operation, Plan, PlanStep, StepScope},
};
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn streamed_count_left_and_held_count_right_join_under_one_owner() {
    fn plan(reverse: bool) -> Plan {
        let relation = Relation::LanguageTarget;
        let counted = Input::Step(StepId(2));
        let source = Input::Relation(relation);
        Plan {
            observation_inputs: Vec::new(),
            observations: Vec::new(),
            parameters: Vec::new(),
            steps: vec![
                PlanStep {
                    scope: StepScope::PerFile,
                    operation: Operation::Select {
                        from: source,
                        comparisons: Vec::new(),
                        kept: Vec::new(),
                    },
                },
                PlanStep {
                    scope: StepScope::PerFile,
                    operation: Operation::GroupCount {
                        from: Input::Step(StepId(0)),
                        keys: vec![ColumnIndex(0)],
                        distinct: None,
                    },
                },
                PlanStep {
                    scope: StepScope::PerFile,
                    operation: Operation::Select {
                        from: Input::Step(StepId(1)),
                        comparisons: Vec::new(),
                        kept: Vec::new(),
                    },
                },
                PlanStep {
                    scope: StepScope::PerFile,
                    operation: Operation::Join {
                        left: if reverse { source } else { counted },
                        right: if reverse { counted } else { source },
                        left_on: vec![ColumnIndex(0)],
                        right_on: vec![ColumnIndex(0)],
                        output: vec![JoinColumn {
                            side: Side::Left,
                            column: ColumnIndex(0),
                        }],
                        filter: Vec::new(),
                        view: Vec::new(),
                        reach: None,
                    },
                },
            ],
            emit: PlanEmit {
                step: StepId(3),
                subject: ColumnIndex(0),
                key: Vec::new(),
                headings: Vec::new(),
            },
            frontier: Vec::new(),
            comparison_origins: Vec::new(),
        }
    }

    let relation = Relation::LanguageTarget;
    let columns = [
        FactColumn(vec![
            to_bits(Value::File(FileId(1))).0,
            to_bits(Value::File(FileId(2))).0,
        ]),
        FactColumn(vec![
            to_bits(Value::Number(Integer(3))).0,
            to_bits(Value::Number(Integer(5))).0,
        ]),
        FactColumn(vec![
            to_bits(Value::Number(Integer(7))).0,
            to_bits(Value::Number(Integer(9))).0,
        ]),
    ];
    let read = |asked| {
        if asked == relation {
            Ok(Rows::Stored(ColumnsView {
                kinds: ryo_relations::domain::relations::service::kinds(relation),
                columns: &columns,
            }))
        } else {
            Err(OperatingError::NotRead)
        }
    };
    let engine = super::super::execution::CommonOperatorEngine::fixture();
    for reverse in [false, true] {
        let plan = plan(reverse);
        let operators =
            crate::domain::operator::contract::types::RequestOperators::prepare_with_schemas(
                &[plan.clone()],
                &[vec![]],
                &[],
                &ryo_relations::domain::relations::service::kinds,
                (std::sync::Arc::new(
                    ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
                )
                    as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
            )
            .unwrap();
        let graph = Arc::new(crate::domain::file_program::service::compile_prepared(
            &[plan],
            &operators,
        ));
        let root = graph.boundaries[0].last().unwrap().1;
        assert_eq!(
            matches!(
                graph.nodes[root.0].inputs[0],
                crate::domain::file_program::types::NodeInput::Node(_)
            ),
            !reverse
        );
        let fixture =
            super::super::testing::FileScope::new(graph, &engine, |_| true, &[relation], &[])
                .unwrap();
        let mut symbols = Symbols::default();
        let mut scratch = OperatorScratch::default();
        let mut memo = Vec::new();
        let mut context = OperatorContext {
            phase: crate::domain::operating::types::AfterTheFiles::Before,
            semantic: None,
            prepared_matchers: None,
            prepared_kinds: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
            source: SourceView(&[]),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        let dag = &fixture.execution;
        dag.begin_rule(0).unwrap();
        let published = dag
            .demand(root, &read, fixture.scope.executor(), &mut context)
            .unwrap();
        let rows = dag
            .relation_owner()
            .resolve(published.as_ref(), &read)
            .unwrap();
        assert_eq!(crate::domain::table_model::service::count_of(&rows).0, 2);
    }
}

#[derive(Debug)]
struct CountingExecutor {
    calls: AtomicUsize,
    refuse: bool,
    native_calls: AtomicUsize,
    native_refuse: bool,
}
impl PrimitiveExecutor for CountingExecutor {
    fn derived_port(&self) -> Arc<dyn crate::outbound::derived::interface::DerivedPort> {
        (std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>)
    }
    fn native_classes(
        &self,
        owner: &std::rc::Rc<super::super::relations::RelationOwner>,
        declaration: &super::super::executor::NativeClassBatch<'_>,
        shared: &mut crate::domain::shared_walk::types::MergedWalk,
        walking: crate::domain::shared_walk::types::Walking<'_>,
    ) -> Result<(), OperatingError> {
        self.native_calls.fetch_add(1, Ordering::SeqCst);
        if self.native_refuse {
            return Err(OperatingError::NotRun);
        }
        super::super::execution::CommonOperatorEngine::fixture().native_classes(
            owner,
            declaration,
            shared,
            walking,
        )
    }
    fn ordered_existence(
        &self,
        owner: &std::rc::Rc<super::super::relations::RelationOwner>,
        shape: &crate::domain::ordered_existence::types::ResolvedShape,
        source: &super::super::relations::PublishedInput,
        rows: crate::domain::operator::contract::views::BorrowedRows<'_>,
        kinds: &[CellKind],
        context: &mut OperatorContext<'_>,
        room: &mut Room,
    ) -> Result<super::super::ordered_existence::StrategyOutcome, OperatingError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.refuse {
            return Err(OperatingError::NotRun);
        }
        super::super::execution::CommonOperatorEngine::fixture()
            .ordered_existence(owner, shape, source, rows, kinds, context, room)
    }
    fn stream(
        &self,
        owner: &std::rc::Rc<super::super::relations::RelationOwner>,
        contract: &crate::domain::operator::contract::types::OperatorContract,
        feed: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.refuse {
            return Err(OperatingError::NotRun);
        }
        super::super::execution::CommonOperatorEngine::fixture()
            .stream(owner, contract, feed, context, room, emit)
    }
    fn stream_representation(
        &self,
        contract: &crate::domain::operator::contract::types::OperatorContract,
        physical: &crate::domain::table_model::collections::representations::publication::PreparedPhysical,
        feed: &mut GraphInputs<'_, '_>,
        owner: &std::rc::Rc<super::super::relations::RelationOwner>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.refuse {
            return Err(OperatingError::NotRun);
        }
        super::super::execution::CommonOperatorEngine::fixture()
            .stream_representation(contract, physical, feed, owner, context, room, emit)
    }
    fn keep_representation(
        &self,
        contract: &crate::domain::operator::contract::types::OperatorContract,
        physical: &crate::domain::table_model::collections::representations::publication::PreparedPhysical,
        feed: &mut GraphInputs<'_, '_>,
        owner: &std::rc::Rc<super::super::relations::RelationOwner>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
    ) -> Result<super::super::relations::PublishedInput, OperatingError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.refuse {
            return Err(OperatingError::NotRun);
        }
        super::super::execution::CommonOperatorEngine::fixture()
            .keep_representation(contract, physical, feed, owner, context, room)
    }
    fn collect(
        &self,
        owner: &std::rc::Rc<super::super::relations::RelationOwner>,
        contract: &crate::domain::operator::contract::types::OperatorContract,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.refuse {
            return Err(OperatingError::NotRun);
        }
        super::super::execution::CommonOperatorEngine::fixture()
            .collect(owner, contract, inputs, context, out)
    }
}

#[test]
fn dag_root_and_recursive_child_use_injected_executor() {
    let relation = Relation::LanguageTarget;
    let plan = Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: vec![],
        steps: vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: Input::Relation(relation),
                    comparisons: vec![],
                    kept: vec![],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: Input::Step(StepId(0)),
                    columns: vec![ColumnIndex(0)],
                    distinct: RepeatedRows::Kept,
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: Input::Step(StepId(1)),
                    columns: vec![ColumnIndex(0)],
                    distinct: RepeatedRows::Kept,
                },
            },
        ],
        emit: PlanEmit {
            step: StepId(2),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
        comparison_origins: vec![],
    };
    let operators =
        crate::domain::operator::contract::types::RequestOperators::prepare_with_schemas(
            &[plan.clone()],
            &[vec![]],
            &[],
            &ryo_relations::domain::relations::service::kinds,
            (std::sync::Arc::new(
                ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
            ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
        )
        .unwrap();
    let graph = Arc::new(crate::domain::file_program::service::compile_prepared(
        &[plan],
        &operators,
    ));
    let root = graph.boundaries[0].last().unwrap().1;
    let columns = [
        FactColumn(vec![to_bits(Value::File(FileId(1))).0]),
        FactColumn(vec![to_bits(Value::Number(Integer(3))).0]),
        FactColumn(vec![to_bits(Value::Number(Integer(10))).0]),
    ];
    let read = |asked| {
        if asked == relation {
            Ok(Rows::Stored(ColumnsView {
                kinds: ryo_relations::domain::relations::service::kinds(relation),
                columns: &columns,
            }))
        } else {
            Err(OperatingError::NotRead)
        }
    };
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_matchers: None,
        prepared_kinds: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let refusing = CountingExecutor {
        calls: AtomicUsize::new(0),
        native_calls: AtomicUsize::new(0),
        native_refuse: false,
        refuse: true,
    };
    let refusing_scope = super::super::testing::FileScope::new(
        Arc::clone(&graph),
        &refusing,
        |_| true,
        &[relation],
        &[],
    )
    .unwrap();
    let dag = &refusing_scope.execution;
    dag.begin_rule(0).unwrap();
    assert!(matches!(
        dag.demand(root, &read, refusing_scope.scope.executor(), &mut context),
        Err(OperatingError::NotRun)
    ));
    assert_eq!(refusing.calls.load(Ordering::SeqCst), 1);
    assert!(matches!(
        dag.demand(root, &read, refusing_scope.scope.executor(), &mut context),
        Err(OperatingError::NotRun)
    ));
    assert_eq!(
        refusing.calls.load(Ordering::SeqCst),
        1,
        "semantic failure is stable within a generation"
    );

    let counting = CountingExecutor {
        calls: AtomicUsize::new(0),
        native_calls: AtomicUsize::new(0),
        native_refuse: false,
        refuse: false,
    };
    let fixture =
        super::super::testing::FileScope::new(graph, &counting, |_| true, &[relation], &[])
            .unwrap();
    let dag = &fixture.execution;
    dag.begin_rule(0).unwrap();
    // A downstream failure must not poison the shared producer. Retry the
    // actual streamed graph path with the same generation and invocation owner.
    let mut rejecting =
        crate::domain::replay::types::Pipe(|_ctx: &mut OperatorContext<'_>, _row: &[Value]| {
            Err(OperatingError::NotRead)
        });
    assert!(matches!(
        FileDagExecution::stream_scoped(
            dag,
            root,
            &read,
            fixture.scope.executor(),
            &mut context,
            &mut rejecting
        ),
        Err(OperatingError::NotRead)
    ));
    assert!(dag.borrow().failed[root.0].is_none());
    assert!(dag.borrow().visiting.iter().all(|running| !running));
    let mut delivered = 0;
    let mut accepting =
        crate::domain::replay::types::Pipe(|_ctx: &mut OperatorContext<'_>, _row: &[Value]| {
            delivered += 1;
            Ok(())
        });
    FileDagExecution::stream_scoped(
        dag,
        root,
        &read,
        fixture.scope.executor(),
        &mut context,
        &mut accepting,
    )
    .unwrap();
    assert_eq!(delivered, 1);
    let result = dag
        .demand(root, &read, fixture.scope.executor(), &mut context)
        .unwrap();
    let rows = dag
        .relation_owner()
        .resolve(result.as_ref(), &read)
        .unwrap();
    assert_eq!(
        crate::domain::table_model::service::count_of(&rows),
        RowCount(1)
    );
    assert!(
        counting.calls.load(Ordering::SeqCst) >= 2,
        "the recursive source node must use the same injected executor"
    );
}

#[test]
fn native_batch_refusal_cannot_publish_classes_or_nodes() {
    use crate::domain::shared_walk::types::{MergedWalk, SharedClasses, Walking};
    use ryo_values::domain::plan_parts::types::{
        Comparison, LiteralText, Operand, QueryLiteral, Test,
    };
    let plan = Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        comparison_origins: vec![],
        frontier: vec![],
        steps: vec![PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::SyntaxTree),
                kept: vec![],
                comparisons: vec![Comparison {
                    left: Operand::Column(ColumnIndex(1)),
                    test: Test::Equal,
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("function".into()))),
                    origin: None,
                }],
            },
        }],
        emit: PlanEmit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
    };
    let plans = [plan];
    let bindings = [vec![]];
    let request = crate::domain::operator::contract::types::RequestOperators::prepare(
        &plans,
        &bindings,
        &[],
        (std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    )
    .unwrap();
    let graph = Arc::new(crate::domain::file_program::service::compile_prepared(
        &plans, &request,
    ));
    let refusing = CountingExecutor {
        calls: AtomicUsize::new(0),
        native_calls: AtomicUsize::new(0),
        refuse: false,
        native_refuse: true,
    };
    let fixture = super::super::testing::FileScope::new(
        graph,
        &refusing,
        |_| true,
        &[Relation::SyntaxTree],
        &[],
    )
    .unwrap();
    let mut merge = MergedWalk::default();
    let mut classes = SharedClasses::default();
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mask = crate::domain::placement::types::Mask::default();
    let result = FileDagExecution::prepare_native(
        &fixture.execution,
        &mut merge,
        Walking {
            plans: &plans,
            bindings: &bindings,
            paths: &[],
            mask: &mask,
            file: FileId(0),
            elements: &elements,
            symbols: &mut symbols,
            source: SourceView(&[]),
            scratch: &mut scratch,
            room: &mut classes,
            steps: crate::domain::operator::types::TreeSteps::Lowered,
        },
        crate::domain::column_kernel::types::KernelStrategy::Existing,
        crate::domain::lowering::types::Counting::DoesNotCount,
        fixture.scope.executor(),
    );
    assert!(matches!(result, Err(OperatingError::NotRun)));
    assert_eq!(refusing.native_calls.load(Ordering::SeqCst), 1);
    assert_eq!(refusing.calls.load(Ordering::SeqCst), 0);
    let state = fixture.execution.borrow();
    assert!(state.classes.classes.is_empty());
    assert!(state.required.iter().all(Option::is_none));
    assert!(state.ready.iter().all(Option::is_none));
    assert_eq!(state.executions.iter().sum::<u64>(), 0);
}
