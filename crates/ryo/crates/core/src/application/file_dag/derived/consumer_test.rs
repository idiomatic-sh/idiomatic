//! Independent rules share one computed producer's equality view through real DAG execution.
use super::indexes::LIMIT;
use super::*;
use crate::application::file_dag::{interface::DagExecution, types::FileDagExecution};
use crate::domain::{
    file_program::service::compile_prepared,
    operating::types::OperatorContext,
    operator::contract::types::RequestOperators,
    table_model::types::{Rows, Table},
    table_scratch::types::OperatorScratch,
    table_texts::types::SourceView,
};
use ryo_language::domain::{
    relations::{
        service::{kinds, to_bits},
        types::{CellKind, ColumnsView, FactColumn, FileId, Integer, Relation, Value},
    },
    symbols::types::Symbols,
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, JoinColumn, RepeatedRows, Side, StepId},
    plans::types::{Emit, Operation, Plan, PlanStep, StepScope},
};
use std::sync::Arc;
fn column(at: u32) -> ColumnIndex {
    ColumnIndex(at)
}
fn plan(swapped: bool) -> Plan {
    let operations = vec![
        Operation::Project {
            from: Input::Relation(Relation::LanguageTarget),
            columns: vec![column(1), column(2)],
            distinct: RepeatedRows::Kept,
        },
        Operation::Union {
            left: Input::Step(StepId(0)),
            right: Input::Step(StepId(0)),
        },
        Operation::Join {
            left: Input::Step(StepId(0)),
            right: Input::Step(StepId(1)),
            left_on: vec![column(0)],
            right_on: vec![column(0)],
            filter: vec![],
            view: vec![],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: column(u32::from(swapped)),
                },
                JoinColumn {
                    side: Side::Right,
                    column: column(u32::from(!swapped)),
                },
            ],
            reach: None,
        },
        Operation::Project {
            from: Input::Step(StepId(2)),
            columns: vec![column(0)],
            distinct: RepeatedRows::Kept,
        },
    ];
    Plan {
        parameters: vec![],
        steps: operations
            .into_iter()
            .map(|operation| PlanStep {
                scope: StepScope::PerFile,
                operation,
            })
            .collect(),
        emit: Emit {
            step: StepId(3),
            subject: column(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
        comparison_origins: vec![],
    }
}
fn number(value: i64) -> Value {
    Value::Number(Integer(value))
}
#[test]
fn different_rule_consumers_share_the_canonical_computed_producer_index() {
    let plans = [plan(false), plan(true)];
    let request = RequestOperators::prepare(&plans, &[vec![], vec![]], &[]).unwrap();
    let graph = Arc::new(compile_prepared(&plans, &request));
    assert_eq!(
        graph.cuts[0][1], graph.cuts[1][1],
        "same actual computed producer"
    );
    assert_ne!(
        graph.cuts[0][2], graph.cuts[1][2],
        "different output semantics must execute both consumers"
    );
    let roots = [graph.cuts[0][2].unwrap(), graph.cuts[1][2].unwrap()];
    for retained_budget in [0, LIMIT] {
        let dag = std::cell::RefCell::new(FileDagExecution::new(
            (graph).clone(),
            |_| true,
            &[Relation::LanguageTarget],
        ));
        let columns = [
            FactColumn(vec![to_bits(Value::File(FileId(1))).0; 2]),
            FactColumn(vec![to_bits(number(2)).0, to_bits(number(3)).0]),
            FactColumn(vec![to_bits(number(9)).0, to_bits(number(7)).0]),
        ];
        let read = |relation| {
            if relation == Relation::LanguageTarget {
                Ok(Rows::Stored(ColumnsView {
                    kinds: kinds(relation),
                    columns: &columns,
                }))
            } else {
                Err(crate::domain::operating::errors::OperatingError::NotRead)
            }
        };
        let (mut symbols, mut scratch, mut memo) =
            (Symbols::default(), OperatorScratch::default(), vec![]);
        let owner = dag.relation_owner();
        owner.derived.prepare_file(
            true,
            &[Relation::LanguageTarget],
            &[],
            &[],
            &plans,
            &crate::domain::operator::types::PreparedPool::default(),
            |_| true,
        );
        owner.derived.set_optional_limit(retained_budget).unwrap();
        let mut context = OperatorContext {
            prepared_kinds: None,
            prepared_matchers: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
            source: SourceView(&[]),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        let mut outputs = Vec::<Table>::new();
        for (rule, root) in roots.into_iter().enumerate() {
            dag.begin_rule(rule).unwrap();
            let result = dag
                .demand(
                    root,
                    &read,
                    &crate::application::file_dag::execution::CommonOperatorEngine,
                    &mut context,
                )
                .unwrap();
            let rows = dag
                .relation_owner()
                .resolve(result.as_ref(), &read)
                .unwrap();
            let mut table = Table::default();
            crate::domain::table_model::service::materialize(&rows, &mut table, &mut Vec::new());
            outputs.push(table);
            if rule == 0 {
                assert!(
                    dag.relation_owner().derived.working_bytes() > 0,
                    "future live consumer owns the required index even without optional payload budget"
                );
            }
        }
        assert_eq!(outputs[0].kinds, vec![CellKind::Number; 2]);
        assert_eq!(outputs[0].cells, [2, 9, 3, 7].map(number));
        assert_eq!(outputs[1].cells, [9, 2, 7, 3].map(number));
        let (builds, hits, _) = dag.relation_owner().derived.counts();
        assert_eq!(
            builds, 1,
            "one equality construction for both real rule invocations"
        );
        assert!(hits >= 1);
        assert!(dag.relation_owner().derived.peak_working_bytes() > 0);
        if retained_budget == 0 {
            assert_eq!(
                dag.relation_owner().derived.owned_bytes(),
                0,
                "last consumer releases the unretained index"
            );
        } else {
            assert!(dag.relation_owner().derived.owned_bytes() > 0);
        }
        dag.borrow_mut().release_population();
        assert_eq!(dag.relation_owner().derived.owned_bytes(), 0);
    }
}
