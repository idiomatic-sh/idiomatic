//! Production-route probes supplement class fixtures; no catalog-only admission is counted.
use super::{execution::CommonOperatorEngine, types::FileDagExecution};
use crate::application::file_dag::acceptance::operator_contract_test::fixtures;
use crate::application::file_dag::cross_execution::evaluate_cross_with_executor;
use crate::application::file_dag::executor::PrimitiveExecutor;
use crate::application::file_dag::feed::Feed;
use crate::application::file_dag::pipeline::service::evaluate_file_with_executor;
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
use crate::domain::table_model::types::{ParameterBinding, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use fixtures::{SOURCE, fixture};
use ryo_language::domain::{
    elements::{
        service::{enter_node, leave_node, table_builder, token},
        types::Elements,
    },
    relations::{
        service::{kinds, to_bits},
        types::{
            CellKind, ColumnsView, FactColumn, FileId, Integer, KindBits, Offset, Relation, Value,
        },
    },
    symbols::types::{ByteSlice, Symbols},
};
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
use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};
#[derive(Debug, Default)]
struct CountingExecutor {
    calls: Mutex<Vec<OperationKind>>,
    emitted: Mutex<Vec<(OperationKind, usize)>>,
}
impl PrimitiveExecutor for CountingExecutor {
    fn stream(
        &self,
        contract: &OperatorContract,
        feed: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        self.calls.lock().unwrap().push(contract.kind);
        CommonOperatorEngine.stream(
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
        contract: &OperatorContract,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        self.calls.lock().unwrap().push(contract.kind);
        CommonOperatorEngine.collect(contract, inputs, context, out)?;
        self.emitted
            .lock()
            .unwrap()
            .push((contract.kind, out.count.0));
        Ok(())
    }
}
fn plan(operation: Operation, scope: StepScope) -> Plan {
    Plan {
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
    )
    .unwrap();
    let graph = Arc::new(compile_prepared(std::slice::from_ref(&plan), &prepared));
    assert!(graph.cuts[0][0].is_none());
    let dag = FileDagExecution::new(graph, |_| true, &[relation]);
    assert!(dag.owned(0, 0).is_none());
}
#[test]
fn all24_actual_production_routes_invoke_and_emit_through_the_required_executor() {
    let executor = CountingExecutor::default();
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
            )
            .unwrap();
            let graph = Arc::new(compile_prepared(std::slice::from_ref(&plan), &prepared));
            assert!(graph.cuts[0][0].is_none(), "Hosted remains cross-file");
            let dag = FileDagExecution::new(graph, |_| true, &[]);
            assert!(dag.owned(0, 0).is_none());
            evaluate_cross_with_executor(
                &plan,
                vec![],
                &Partials::default(),
                &mut context,
                &mut [RowsFed::default()],
                &case.inputs,
                &executor,
                &[contract],
            )
            .unwrap()
        } else {
            let relations = named(&inputs_of(&case.operation))
                .iter()
                .map(|input| match input {
                    Input::Relation(relation) => *relation,
                    Input::Step(_) => panic!("single-step fixture"),
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
            let output_columns = (0..case.expected.kinds.len())
                .map(|column| ColumnIndex(column as u32))
                .collect();
            let plan = Plan {
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
            )
            .unwrap();
            let graph = Arc::new(compile_prepared(std::slice::from_ref(&plan), &prepared));
            assert!(graph.cuts[0][0].is_some(), "{kind:?} must reach the DAG");
            assert!(graph.cuts[0][1].is_none(), "emit stays residual");
            let dag = RefCell::new(FileDagExecution::new(graph, |_| true, &relations));
            assert!(
                dag.borrow().owned(0, 0).is_some(),
                "{kind:?} owned before execution"
            );
            assert!(dag.borrow().owned(0, 1).is_none());
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
                &[],
                None,
                0,
                Some(&dag),
                &executor,
                &prepared.rules[0],
            )
            .unwrap();
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
    let contract = Arc::new(
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
    evaluate_file_with_executor(
        &plan,
        &read,
        None,
        &mut context,
        &mut pool,
        &mut PreparedPool::default(),
        &[],
        None,
        0,
        None,
        executor,
        &[contract],
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
struct RejectingExecutor;
impl PrimitiveExecutor for RejectingExecutor {
    fn stream(
        &self,
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
    let contract = Arc::new(
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
    assert_eq!(
        evaluate_file_with_executor(
            &plan,
            &read,
            None,
            &mut context,
            &mut pool_for(std::slice::from_ref(&plan)),
            &mut PreparedPool::default(),
            &[],
            None,
            0,
            None,
            &RejectingExecutor,
            &[contract]
        ),
        Err(OperatingError::WrongOperation)
    );
}

mod generations;
