//! A DAG demand must use the same injected primitive executor as pipeline steps.
use super::*;
use crate::application::file_dag::executor::PrimitiveExecutor;
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{ColumnsView, FactColumn, FileId, Integer},
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, RepeatedRows, StepId},
    plans::types::{Emit as PlanEmit, Operation, Plan, PlanStep, StepScope},
};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
struct CountingExecutor {
    calls: AtomicUsize,
    refuse: bool,
}
impl PrimitiveExecutor for CountingExecutor {
    fn derived_port(&self) -> std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort> {
        crate::outbound::derived::factory::test_port()
    }
    fn stream(
        &self,
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
            .stream(contract, feed, context, room, emit)
    }
    fn collect(
        &self,
        contract: &crate::domain::operator::contract::types::OperatorContract,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        super::super::execution::CommonOperatorEngine::fixture()
            .collect(contract, inputs, context, out)
    }
}

#[test]
fn dag_root_and_recursive_child_use_injected_executor() {
    let relation = Relation::LanguageTarget;
    let plan = Plan {
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
            crate::outbound::derived::factory::test_port(),
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
        refuse: true,
    };
    let dag = std::cell::RefCell::new(FileDagExecution::new(
        (graph).clone(),
        |_| true,
        &[relation],
    ));
    dag.begin_rule(0);
    assert!(matches!(
        dag.demand(root, &read, &refusing, &mut context),
        Err(OperatingError::NotRun)
    ));
    assert_eq!(refusing.calls.load(Ordering::SeqCst), 1);

    let counting = CountingExecutor {
        calls: AtomicUsize::new(0),
        refuse: false,
    };
    let dag = std::cell::RefCell::new(FileDagExecution::new(graph, |_| true, &[relation]));
    dag.begin_rule(0);
    let result = dag.demand(root, &read, &counting, &mut context).unwrap();
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
