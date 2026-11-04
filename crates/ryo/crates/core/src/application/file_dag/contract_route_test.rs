//! Production-route probes supplement class fixtures; no catalog-only admission is counted.
use super::execution::CommonOperatorEngine;
use crate::application::file_dag::acceptance::operator_contract_test::fixtures;
use crate::application::file_dag::cross_execution::evaluate_cross_with_executor;
use crate::application::file_dag::executor::NativeClassBatch;
use crate::application::file_dag::executor::PrimitiveExecutor;
use crate::application::file_dag::feed::Feed;
use crate::application::file_dag::pipeline::service::evaluate_file_with_executor;
use crate::application::file_dag::relations::RelationOwner;
use crate::application::file_dag::service::GraphInputs;
use crate::domain::coverage::types::{Lacking, Partial, Partials};
use crate::domain::features::service::kind_of;
use crate::domain::file_program::service::compile_prepared;
use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, OperatorContext, Room},
};
use crate::domain::operation_schema::types::InputSchema;
use crate::domain::operator::service::registered;
use crate::domain::operator::{
    contract::types::{OperatorContract, RequestOperators},
    service::pool_for,
    types::{PreparedPool, RowsFed},
};
use crate::domain::replay::types::Pipe;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::types::{ParameterBinding, Rows};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use fixtures::{SOURCE, fixture};
use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::{
    elements::{
        service::{enter_node, leave_node, table_builder, token},
        types::Elements,
    },
    symbols::types::{ByteSlice, Symbols},
};
use ryo_relations::domain::relations::{
    service::{kinds, to_bits},
    types::{
        CellKind, ColumnsView, FactColumn, FileId, Integer, KindBits, Offset, Relation, Value,
    },
};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation},
    plan_parts::types::{
        ColumnIndex, Comparison, Input, Operand, ParameterId, RepeatedRows, StepId, Test,
        WholeNumber,
    },
    plans::{
        service::{inputs_of, named},
        types::{Emit as PlanEmit, Operation, OperationKind, Plan, PlanStep, StepScope},
    },
    stored_facts::types::PartialCause,
};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
#[derive(Debug)]
struct CountingExecutor {
    engine: CommonOperatorEngine<'static>,
    ordered_calls: std::sync::atomic::AtomicUsize,
    decline_ordered: bool,
    calls: Mutex<Vec<OperationKind>>,
    emitted: Mutex<Vec<(OperationKind, usize)>>,
}
impl Default for CountingExecutor {
    fn default() -> Self {
        Self {
            engine: CommonOperatorEngine::fixture(),
            ordered_calls: std::sync::atomic::AtomicUsize::default(),
            decline_ordered: false,
            calls: Mutex::default(),
            emitted: Mutex::default(),
        }
    }
}
impl PrimitiveExecutor for CountingExecutor {
    fn keep_representation(
        &self,
        contract: &OperatorContract,
        _physical: &crate::domain::table_model::collections::representations::publication::PreparedPhysical,
        feed: &mut GraphInputs<'_, '_>,
        owner: &Rc<RelationOwner>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
    ) -> Result<crate::application::file_dag::relations::PublishedInput, OperatingError> {
        crate::application::file_dag::executor::keep_rows(
            self, contract, feed, owner, context, room,
        )
    }
    fn ordered_existence(
        &self,
        owner: &std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
        shape: &crate::domain::ordered_existence::types::ResolvedShape,
        source: &crate::application::file_dag::relations::PublishedInput,
        rows: crate::domain::operator::contract::views::BorrowedRows<'_>,
        output_kinds: &[ryo_relations::domain::relations::types::CellKind],
        context: &mut OperatorContext<'_>,
        room: &mut Room,
    ) -> Result<crate::application::file_dag::ordered_existence::StrategyOutcome, OperatingError>
    {
        self.ordered_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.calls.lock().unwrap().push(OperationKind::Select);
        if self.decline_ordered {
            return Ok(crate::application::file_dag::ordered_existence::StrategyOutcome::Declined(crate::application::file_dag::ordered_existence::AdmissionDecline::UnprovenPopulation));
        }
        self.engine
            .ordered_existence(owner, shape, source, rows, output_kinds, context, room)
    }

    fn native_classes(
        &self,
        owner: &Rc<RelationOwner>,
        declaration: &NativeClassBatch<'_>,
        shared: &mut crate::domain::shared_walk::types::MergedWalk,
        walking: crate::domain::shared_walk::types::Walking<'_>,
    ) -> Result<(), OperatingError> {
        self.engine
            .native_classes(owner, declaration, shared, walking)
    }
    fn stream_representation(
        &self,
        contract: &OperatorContract,
        physical: &crate::domain::table_model::collections::representations::publication::PreparedPhysical,
        feed: &mut GraphInputs<'_, '_>,
        owner: &Rc<RelationOwner>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        self.calls.lock().unwrap().push(contract.kind);
        self.engine.stream_representation(
            contract,
            physical,
            feed,
            owner,
            context,
            room,
            &mut Pipe(|context: &mut OperatorContext<'_>, row: &[Value]| {
                self.emitted.lock().unwrap().push((contract.kind, 1));
                emit.row(context, row)
            }),
        )
    }
    fn derived_port(&self) -> std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort> {
        self.engine.derived_port()
    }
    fn stream(
        &self,
        owner: &Rc<RelationOwner>,
        contract: &OperatorContract,
        feed: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        self.calls.lock().unwrap().push(contract.kind);
        self.engine.stream(
            owner,
            contract,
            feed,
            context,
            room,
            &mut Pipe(|context: &mut OperatorContext<'_>, row: &[Value]| {
                self.emitted.lock().unwrap().push((contract.kind, 1));
                emit.row(context, row)
            }),
        )
    }
    fn collect(
        &self,
        owner: &Rc<RelationOwner>,
        contract: &OperatorContract,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        self.calls.lock().unwrap().push(contract.kind);
        self.engine.collect(owner, contract, inputs, context, out)?;
        self.emitted
            .lock()
            .unwrap()
            .push((contract.kind, out.count.0));
        Ok(())
    }
}
fn plan(operation: Operation, scope: StepScope) -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: vec![],
        steps: vec![PlanStep { operation, scope }],
        emit: PlanEmit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
        comparison_origins: vec![],
    }
}
// Provider schemas are static catalog metadata; fixture tables retain owned cells.
fn stored_kinds(table: &Table) -> &'static [CellKind] {
    use CellKind::{Node, Number, Span, Symbol};
    match table.kinds.as_slice() {
        [Number] => &[Number],
        [Number, Number] => &[Number, Number],
        [Number, Number, Number, Number] => &[Number, Number, Number, Number],
        [Symbol] => &[Symbol],
        [Symbol, Span] => &[Symbol, Span],
        [Node] => &[Node],
        [Node, Node] => &[Node, Node],
        [Span] => &[Span],
        [Span, Span] => &[Span, Span],
        unexpected => panic!("unsupported provider fixture schema: {unexpected:?}"),
    }
}
fn columns(table: &Table) -> Vec<FactColumn> {
    (0..table.kinds.len())
        .map(|column| {
            FactColumn(
                (0..table.count.0)
                    .map(|row| to_bits(table.cells[row * table.kinds.len() + column]).0)
                    .collect(),
            )
        })
        .collect()
}
fn actual_tree() -> (Elements, Symbols) {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(SOURCE));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(1));
    enter_node(&mut builder, KindBits(3), Offset(1));
    token(&mut builder, KindBits(2), Offset(1), Offset(2));
    leave_node(&mut builder, Offset(2));
    leave_node(&mut builder, Offset(2));
    drop(builder);
    for (kind, name) in ["unused", "root", "token", "branch"]
        .into_iter()
        .enumerate()
    {
        name_kind(
            &mut completion(&mut elements, &mut symbols, ByteSlice(SOURCE), FileId(0)),
            KindBits(kind as u16),
            NameText(name),
        );
    }
    (elements, symbols)
}
#[test]
fn a_single_emitted_step_is_residual_not_a_dag_cut() {
    let case = fixture(OperationKind::Select);
    let relation = Relation::Function;
    let plan = plan(case.operation, StepScope::PerFile);
    let prepared = RequestOperators::prepare_with_schemas(
        std::slice::from_ref(&plan),
        std::slice::from_ref(&case.bindings),
        &[],
        &|asked| {
            assert_eq!(asked, relation);
            stored_kinds(&case.inputs[0])
        },
        crate::outbound::derived::factory::test_port(),
    )
    .unwrap();
    let graph = Arc::new(compile_prepared(std::slice::from_ref(&plan), &prepared));
    assert!(graph.cuts[0][0].is_none());
    let scope = crate::application::file_dag::testing::FileScope::new(
        graph,
        CommonOperatorEngine::fixture_ref(),
        |_| true,
        &[relation],
        &[],
    )
    .unwrap();
    let dag = scope.execution.borrow();
    assert!(dag.graph.cuts[0][0].is_none());
}
#[test]
fn all26_actual_production_routes_invoke_and_emit_through_the_required_executor() {
    let executor = CountingExecutor::default();
    let semantic = fixtures::KnownSyntax;
    for kind in OperationKind::ALL {
        let mut case = fixture(*kind);
        let schemas = case
            .inputs
            .iter()
            .map(|table| InputSchema {
                kinds: &table.kinds,
                tree: None,
            })
            .collect::<Vec<_>>();
        let contract = Arc::new(
            super::derived::prepare::operator(
                registered(*kind),
                &case.operation,
                &schemas,
                &case.bindings,
            )
            .unwrap(),
        );
        let mut scratch = OperatorScratch::default();
        let mut memo = Vec::new();
        let mut context = OperatorContext {
            semantic: Some(&semantic),
            prepared_matchers: None,
            prepared_kinds: None,
            bindings: &case.bindings,
            symbols: (&mut case.symbols).into(),
            source: SourceView(SOURCE),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        let actual = if *kind == OperationKind::Hosted {
            let plan = plan(case.operation, StepScope::CrossFile);
            let hosted = [HostRelation {
                name: HostName("fixture".into()),
                columns: vec![HostColumn {
                    name: HostColumnName("value".into()),
                    kind: HostKind::Number,
                }],
                rows: vec![],
            }];
            let prepared = RequestOperators::prepare_with_schemas(
                std::slice::from_ref(&plan),
                std::slice::from_ref(&case.bindings),
                &hosted,
                &kinds,
                crate::outbound::derived::factory::test_port(),
            )
            .unwrap();
            let graph = Arc::new(compile_prepared(std::slice::from_ref(&plan), &prepared));
            assert!(graph.cuts[0][0].is_none(), "Hosted remains cross-file");
            let scope = crate::application::file_dag::testing::FileScope::new(
                graph,
                &executor,
                |_| true,
                &[],
                &[],
            )
            .unwrap();
            let dag = scope.execution.borrow();
            assert!(dag.graph.cuts[0][0].is_none());
            evaluate_cross_with_executor(
                &plan,
                vec![],
                &Partials::default(),
                &mut context,
                &mut [RowsFed::default()],
                &case.inputs,
                scope.scope.executor(),
                &[contract],
            )
            .unwrap()
        } else {
            let relations = named(&inputs_of(&case.operation))
                .iter()
                .map(|input| match input {
                    Input::Relation(relation) => *relation,
                    Input::Step(_) => panic!("single-step fixture"),
                    Input::BoundObservation(_) => {
                        panic!("ordinary relation fixture has no named bindings")
                    }
                })
                .collect::<Vec<_>>();
            let storage = case.inputs.iter().map(columns).collect::<Vec<_>>();
            let relation_kinds = |relation: Relation| {
                if relation == Relation::SyntaxTree {
                    return kinds(relation);
                }
                let index = relations
                    .iter()
                    .position(|input| *input == relation)
                    .expect("fixture declares every relation input");
                stored_kinds(&case.inputs[index])
            };
            let read = |relation| {
                relations
                    .iter()
                    .position(|input| *input == relation)
                    .map(|index| ColumnsView {
                        kinds: stored_kinds(&case.inputs[index]),
                        columns: &storage[index],
                    })
            };
            let output_columns = (0..contract.collected_kinds.as_ref().unwrap().len())
                .map(|column| ColumnIndex(column as u32))
                .collect();
            let plan = Plan {
                observation_inputs: Vec::new(),
                observations: Vec::new(),
                parameters: vec![],
                steps: vec![
                    PlanStep {
                        operation: case.operation,
                        scope: StepScope::PerFile,
                    },
                    PlanStep {
                        operation: Operation::Project {
                            from: Input::Step(StepId(0)),
                            columns: output_columns,
                            distinct: RepeatedRows::Kept,
                        },
                        scope: StepScope::PerFile,
                    },
                ],
                emit: PlanEmit {
                    step: StepId(1),
                    subject: ColumnIndex(0),
                    key: vec![],
                    headings: vec![],
                },
                frontier: vec![],
                comparison_origins: vec![],
            };
            let prepared = RequestOperators::prepare_with_schemas(
                std::slice::from_ref(&plan),
                std::slice::from_ref(&case.bindings),
                &[],
                &relation_kinds,
                crate::outbound::derived::factory::test_port(),
            )
            .unwrap();
            let graph = Arc::new(compile_prepared(std::slice::from_ref(&plan), &prepared));
            assert!(graph.cuts[0][0].is_some(), "{kind:?} must reach the DAG");
            assert!(graph.cuts[0][1].is_none(), "emit stays residual");
            let scope = crate::application::file_dag::testing::FileScope::new(
                graph,
                &executor,
                |_| true,
                &relations,
                &[],
            )
            .unwrap();
            let dag = &scope.execution;
            assert!(
                dag.borrow().graph.cuts[0][0].is_some(),
                "{kind:?} owned before execution"
            );
            assert!(dag.borrow().graph.cuts[0][1].is_none());
            let (elements, _tree_symbols) = actual_tree();
            let tree = relations
                .contains(&Relation::SyntaxTree)
                .then_some(TreeRows {
                    elements: &elements,
                    file: FileId(0),
                    form: TreeForm::Elements,
                });
            let mut pool = pool_for(std::slice::from_ref(&plan));
            evaluate_file_with_executor(
                &plan,
                &read,
                tree,
                &mut context,
                &mut pool,
                &mut PreparedPool::default(),
                0,
                dag,
                scope.scope.executor(),
            )
            .unwrap_or_else(|error| panic!("DAG-owned {kind:?}: {error:?}"));
            pool.steps.remove(1)
        };
        assert!(actual.count.0 > 0, "actual nonempty {kind:?} route");
        let rows = actual
            .cells
            .chunks_exact(actual.kinds.len())
            .map(<[Value]>::to_vec)
            .collect::<Vec<_>>();
        fixtures::assert_expected(&case.expected, &rows, &case.symbols);
        assert!(executor.calls.lock().unwrap().contains(kind));
        assert!(
            executor
                .emitted
                .lock()
                .unwrap()
                .iter()
                .any(|(seen, count)| seen == kind && *count > 0)
        );
    }
    assert_eq!(
        executor
            .calls
            .lock()
            .unwrap()
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        OperationKind::ALL.len()
    );
}
fn partial_route(
    operation: Operation,
    partial: Relation,
    executor: &dyn PrimitiveExecutor,
) -> Table {
    let case = fixture(kind_of(&operation));
    let schemas = case
        .inputs
        .iter()
        .map(|table| InputSchema {
            kinds: &table.kinds,
            tree: None,
        })
        .collect::<Vec<_>>();
    let _contract = Arc::new(
        super::derived::prepare::operator(
            registered(kind_of(&operation)),
            &operation,
            &schemas,
            &[],
        )
        .unwrap(),
    );
    let inputs = named(&inputs_of(&operation))
        .iter()
        .map(|input| match input {
            Input::Relation(relation) => *relation,
            Input::Step(_) => panic!("relation fixture"),
            Input::BoundObservation(_) => panic!("ordinary relation fixture has no named bindings"),
        })
        .collect::<Vec<_>>();
    let storage = case.inputs.iter().map(columns).collect::<Vec<_>>();
    let read = |relation| {
        inputs
            .iter()
            .position(|input| *input == relation)
            .map(|at| ColumnsView {
                kinds: stored_kinds(&case.inputs[at]),
                columns: &storage[at],
            })
    };
    let plan = plan(operation, StepScope::PerFile);
    let mut pool = pool_for(std::slice::from_ref(&plan));
    pool.partial.push(Partial {
        relation: partial,
        cause: PartialCause("partial provider fixture".into()),
        lacking: Lacking::Any,
        at: None,
    });
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        semantic: None,
        prepared_matchers: None,
        prepared_kinds: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(SOURCE),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let prepared = RequestOperators::prepare_with_schemas(
        std::slice::from_ref(&plan),
        &[vec![]],
        &[],
        &|relation| {
            stored_kinds(
                &case.inputs[inputs
                    .iter()
                    .position(|input| *input == relation)
                    .expect("declared relation")],
            )
        },
        executor.derived_port(),
    )
    .unwrap();
    let scope = crate::application::file_dag::testing::FileScope::new(
        Arc::new(compile_prepared(std::slice::from_ref(&plan), &prepared)),
        executor,
        |_| true,
        &inputs,
        &pool.partial,
    )
    .unwrap();
    evaluate_file_with_executor(
        &plan,
        &read,
        None,
        &mut context,
        &mut pool,
        &mut PreparedPool::default(),
        0,
        &scope.execution,
        scope.scope.executor(),
    )
    .unwrap();
    pool.steps.remove(0)
}
#[test]
fn partial_positive_rows_reach_common_engine_but_partial_absence_does_not() {
    let executor = CountingExecutor::default();
    let select = fixture(OperationKind::Select).operation;
    let positive = partial_route(select, Relation::Function, &executor);
    assert_eq!(positive.count.0, 3);
    assert_eq!(*executor.calls.lock().unwrap(), vec![OperationKind::Select]);
    let difference = fixture(OperationKind::Difference).operation;
    let absent = partial_route(difference, Relation::Scope, &executor);
    assert_eq!(absent.count.0, 0);
    assert_eq!(
        *executor.calls.lock().unwrap(),
        vec![OperationKind::Select],
        "absence proof cannot execute on partial RHS"
    );
}
#[derive(Debug)]
struct RejectingExecutor {
    port: std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>,
}
impl PrimitiveExecutor for RejectingExecutor {
    fn keep_representation(
        &self,
        _: &OperatorContract,
        _: &crate::domain::table_model::collections::representations::publication::PreparedPhysical,
        _: &mut GraphInputs<'_, '_>,
        _: &Rc<RelationOwner>,
        _: &mut OperatorContext<'_>,
        _: &mut Room,
    ) -> Result<crate::application::file_dag::relations::PublishedInput, OperatingError> {
        Err(OperatingError::WrongOperation)
    }
    fn ordered_existence(
        &self,
        _owner: &std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
        shape: &crate::domain::ordered_existence::types::ResolvedShape,
        source: &crate::application::file_dag::relations::PublishedInput,
        rows: crate::domain::operator::contract::views::BorrowedRows<'_>,
        output_kinds: &[ryo_relations::domain::relations::types::CellKind],
        context: &mut OperatorContext<'_>,
        room: &mut Room,
    ) -> Result<crate::application::file_dag::ordered_existence::StrategyOutcome, OperatingError>
    {
        Err(OperatingError::WrongOperation)
    }

    fn native_classes(
        &self,
        _: &Rc<RelationOwner>,
        _: &NativeClassBatch<'_>,
        _: &mut crate::domain::shared_walk::types::MergedWalk,
        _: crate::domain::shared_walk::types::Walking<'_>,
    ) -> Result<(), OperatingError> {
        Err(OperatingError::WrongOperation)
    }
    fn stream_representation(
        &self,
        _: &OperatorContract,
        _: &crate::domain::table_model::collections::representations::publication::PreparedPhysical,
        _: &mut GraphInputs<'_, '_>,
        _: &Rc<RelationOwner>,
        _: &mut OperatorContext<'_>,
        _: &mut Room,
        _: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        Err(OperatingError::WrongOperation)
    }
    fn derived_port(&self) -> std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort> {
        std::sync::Arc::clone(&self.port)
    }
    fn stream(
        &self,
        _owner: &Rc<RelationOwner>,
        _: &OperatorContract,
        _: &mut dyn Feed<'_>,
        _: &mut OperatorContext<'_>,
        _: &mut Room,
        _: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        Err(OperatingError::WrongOperation)
    }
    fn collect(
        &self,
        _owner: &Rc<RelationOwner>,
        _: &OperatorContract,
        _: &[Rows<'_>],
        _: &mut OperatorContext<'_>,
        _: &mut Table,
    ) -> Result<(), OperatingError> {
        Err(OperatingError::WrongOperation)
    }
}
#[test]
fn production_route_cannot_bypass_an_injected_executor_refusal() {
    let case = fixture(OperationKind::Select);
    let _contract = Arc::new(
        super::derived::prepare::operator(
            registered(OperationKind::Select),
            &case.operation,
            &[InputSchema {
                kinds: &case.inputs[0].kinds,
                tree: None,
            }],
            &[],
        )
        .unwrap(),
    );
    let storage = columns(&case.inputs[0]);
    let read = |relation| {
        (relation == Relation::Function).then_some(ColumnsView {
            kinds: stored_kinds(&case.inputs[0]),
            columns: &storage,
        })
    };
    let plan = plan(case.operation, StepScope::PerFile);
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        semantic: None,
        prepared_matchers: None,
        prepared_kinds: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(SOURCE),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let port = crate::outbound::derived::factory::test_port();
    let prepared = RequestOperators::prepare_with_schemas(
        std::slice::from_ref(&plan),
        &[vec![]],
        &[],
        &|_| stored_kinds(&case.inputs[0]),
        std::sync::Arc::clone(&port),
    )
    .unwrap();
    let executor = RejectingExecutor { port };
    let scope = crate::application::file_dag::testing::FileScope::new(
        Arc::new(compile_prepared(std::slice::from_ref(&plan), &prepared)),
        &executor,
        |_| true,
        &[Relation::Function],
        &[],
    )
    .unwrap();
    assert_eq!(
        evaluate_file_with_executor(
            &plan,
            &read,
            None,
            &mut context,
            &mut pool_for(std::slice::from_ref(&plan)),
            &mut PreparedPool::default(),
            0,
            &scope.execution,
            scope.scope.executor()
        ),
        Err(OperatingError::WrongOperation)
    );
}

mod generations;

use ryo_values::domain::rows::types::Table;

#[test]
fn ordered_existence_stream_and_collection_use_the_same_prepared_strategy() {
    use crate::application::file_dag::interface::DagExecution as _;
    use std::sync::atomic::Ordering;
    let (plan, schema) = ordered_strategy_fixture();
    let numbers = [1_i64, 3, 2];
    let columns = schema
        .iter()
        .map(|kind| {
            FactColumn(
                numbers
                    .iter()
                    .map(|number| {
                        let value = match kind {
                            CellKind::Node => {
                                Value::Node(ryo_relations::domain::relations::types::NodeId(0))
                            }
                            CellKind::Symbol => {
                                Value::Symbol(ryo_relations::domain::terms::types::Symbol(0))
                            }
                            CellKind::File => Value::File(FileId(0)),
                            CellKind::Span => {
                                Value::Span(ryo_relations::domain::relations::types::Span {
                                    start: Offset(0),
                                    end: Offset(1),
                                })
                            }
                            CellKind::Number => Value::Number(Integer(*number)),
                            CellKind::Flag => {
                                Value::Flag(ryo_relations::domain::relations::types::Flag::Raised)
                            }
                        };
                        to_bits(value).0
                    })
                    .collect(),
            )
        })
        .collect::<Vec<_>>();
    let read = |relation| {
        if relation == Relation::SourceLocation {
            Ok(Rows::Stored(ColumnsView {
                kinds: kinds(relation),
                columns: &columns,
            }))
        } else {
            Err(OperatingError::NotRead)
        }
    };
    let mut expected = None;
    for stream in [false, true] {
        for decline in [true, false] {
            let executor = CountingExecutor {
                decline_ordered: decline,
                ..CountingExecutor::default()
            };
            let prepared = RequestOperators::prepare(
                std::slice::from_ref(&plan),
                &[vec![]],
                &[],
                executor.derived_port(),
            )
            .unwrap();
            let graph = Arc::new(compile_prepared(std::slice::from_ref(&plan), &prepared));
            let project = graph.program_nodes[0][3]
                .clone()
                .expect("retained projection is scheduled");
            let scope = crate::application::file_dag::testing::FileScope::new(
                graph,
                &executor,
                |_| true,
                &[Relation::SourceLocation],
                &[],
            )
            .unwrap();
            scope.execution.begin_rule(0).unwrap();
            let mut symbols = Symbols::default();
            let mut scratch = OperatorScratch::default();
            let mut memo = Vec::new();
            let mut context = OperatorContext {
                semantic: None,
                prepared_matchers: None,
                prepared_kinds: None,
                bindings: &[],
                symbols: (&mut symbols).into(),
                source: SourceView(b""),
                paths: &[],
                arenas: &[],
                memo: &mut memo,
                scratch: &mut scratch,
            };
            let mut actual = Table::default();
            if stream {
                super::types::FileDagExecution::stream_scoped(
                    &scope.execution,
                    project,
                    &read,
                    scope.scope.executor(),
                    &mut context,
                    &mut crate::domain::replay::types::Collect(&mut actual),
                )
                .unwrap();
            } else {
                let publication = scope
                    .execution
                    .demand(project, &read, scope.scope.executor(), &mut context)
                    .unwrap();
                let rows = scope
                    .scope
                    .owner()
                    .resolve(publication.as_ref(), &read)
                    .unwrap();
                crate::domain::table_model::service::materialize(
                    &rows,
                    &mut actual,
                    &mut Vec::new(),
                );
            }
            assert_eq!(
                actual.count.0, 2,
                "strict order retains the two values below the maximum"
            );
            if let Some(expected) = &expected {
                assert_eq!(
                    &actual, expected,
                    "same exact ordered output across physical routes"
                );
            } else {
                expected = Some(actual);
            }
            assert_eq!(
                executor.ordered_calls.load(Ordering::Relaxed),
                1,
                "each entry consults the shared prepared strategy once"
            );
            let joins = executor
                .calls
                .lock()
                .unwrap()
                .iter()
                .filter(|kind| **kind == OperationKind::Join)
                .count();
            assert_eq!(
                joins > 0,
                decline,
                "eligible strategy skips Join; genuine decline uses the common ordinary operator path"
            );
        }
    }
}

fn ordered_strategy_fixture() -> (Plan, Vec<CellKind>) {
    use ryo_values::domain::plan_parts::types::{JoinColumn, Side};
    let schema = kinds(ryo_relations::domain::relations::types::Relation::SourceLocation).to_vec();
    let number = ColumnIndex(
        schema
            .iter()
            .position(|kind| *kind == CellKind::Number)
            .expect("typed numeric source") as u32,
    );
    let source = Operation::Select {
        from: Input::Relation(ryo_relations::domain::relations::types::Relation::SourceLocation),
        comparisons: vec![],
        kept: vec![],
    };
    let join = Operation::Join {
        left: Input::Step(StepId(0)),
        right: Input::Step(StepId(0)),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        output: vec![
            JoinColumn {
                side: Side::Left,
                column: number,
            },
            JoinColumn {
                side: Side::Right,
                column: number,
            },
        ],
        filter: vec![],
        view: vec![],
        reach: None,
    };
    let select = Operation::Select {
        from: Input::Step(StepId(1)),
        comparisons: vec![Comparison {
            test: Test::LessThan,
            left: Operand::Column(ColumnIndex(0)),
            right: Operand::Column(ColumnIndex(1)),
            origin: None,
        }],
        kept: vec![],
    };
    let project = Operation::Project {
        from: Input::Step(StepId(2)),
        columns: vec![ColumnIndex(0)],
        distinct: RepeatedRows::Dropped,
    };
    let anti = Operation::Antijoin {
        left: Input::Step(StepId(0)),
        right: Input::Step(StepId(3)),
        left_on: vec![number],
        right_on: vec![ColumnIndex(0)],
    };
    (
        Plan {
            observation_inputs: Vec::new(),
            observations: Vec::new(),
            parameters: vec![],
            steps: [source, join, select, project, anti]
                .into_iter()
                .map(|operation| PlanStep {
                    operation,
                    scope: StepScope::PerFile,
                })
                .collect(),
            comparison_origins: vec![],
            emit: PlanEmit {
                step: StepId(4),
                subject: ColumnIndex(0),
                key: vec![],
                headings: vec![],
            },
            frontier: vec![],
        },
        schema,
    )
}
