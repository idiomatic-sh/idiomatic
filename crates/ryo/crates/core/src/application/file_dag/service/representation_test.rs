//! Actual DAG demands retain each bulk representation through the mandatory entry.
use super::*;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::collections::representations::{
    publication::PublishedRelation,
    service::{borrowed, kind, registered},
    types::RepresentationKind,
};
use crate::domain::table_model::types::{SelectedRows, StoredSelection};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::{elements::{
        service::{enter_node, leave_node, table_builder, token},
        types::Elements,
    }, symbols::types::{ByteSlice, Symbols}};
use ryo_relations::domain::relations::{
        service::{kinds, to_bits},
        types::{ColumnsView, FactColumn, FileId, Integer, KindBits, Offset},
    };
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, RepeatedRows, StepId},
    plans::types::{Emit as PlanEmit, Operation, Plan, PlanStep, StepScope},
};

fn plan(relation: Relation) -> Plan {
    Plan {
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
        ],
        emit: PlanEmit {
            step: StepId(1),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
        comparison_origins: vec![],
    }
}

#[test]
fn dag_preserves_all_four_bulk_routes_with_and_without_optional_retention() {
    let numbers = [1, 2, 3];
    let columns =
        numbers.map(|number| FactColumn(vec![to_bits(Value::Number(Integer(number))).0; 2]));
    let source = ColumnsView {
        kinds: &[CellKind::Number; 3],
        columns: &columns,
    };
    let selected = SelectedRows {
        source,
        selection: Arc::new(StoredSelection {
            relation: Relation::LanguageTarget,
            rows: Arc::new(vec![1, 0, 1]),
            columns: vec![ColumnIndex(2), ColumnIndex(1), ColumnIndex(0)],
            kinds: vec![CellKind::Number; 3],
            identity: Arc::new(Default::default()),
        }),
    };
    let computed = Table {
        kinds: vec![CellKind::Number; 3],
        cells: vec![Value::Number(Integer(7)); 6],
        count: RowCount(2),
    };
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"ab"));
    enter_node(&mut builder, KindBits(0), Offset(0));
    token(&mut builder, KindBits(0), Offset(0), Offset(1));
    leave_node(&mut builder, Offset(2));
    drop(builder);
    for variant in RepresentationKind::ALL {
        let (relation, input, count) = match variant {
            RepresentationKind::Stored => (Relation::LanguageTarget, Rows::Stored(source), 2),
            RepresentationKind::Selected => (
                Relation::LanguageTarget,
                Rows::Selected(selected.clone()),
                3,
            ),
            RepresentationKind::Computed => {
                (Relation::LanguageTarget, Rows::Computed(&computed), 2)
            }
            RepresentationKind::Tree => (
                Relation::SyntaxChild,
                Rows::Tree(TreeRows {
                    elements: &elements,
                    file: FileId(3),
                    form: TreeForm::Edges,
                }),
                1,
            ),
        };
        for budget in [0, RESULT_LIMIT] {
            let plans = [plan(relation), plan(relation)];
            let operators =
                crate::domain::operator::contract::types::RequestOperators::prepare_with_schemas(
                    &plans,
                    &[vec![], vec![]],
                    &[],
                    &|_| crate::domain::table_model::service::kinds_of(&input),
                )
                .unwrap();
            let graph = Arc::new(crate::domain::file_program::service::compile_prepared(
                &plans, &operators,
            ));
            let root = graph.boundaries[0]
                .first()
                .expect("internal representation producer feeds the residual terminal")
                .1;
            assert_eq!(graph.cuts[0][0], Some(root));
            let mut dag = FileDagExecution::new(graph, |_| true, &[relation]);
            dag.limit = budget;
            dag.begin_rule(0).unwrap();
            let read = |asked| {
                if asked == relation {
                    Ok(input.clone())
                } else {
                    Err(OperatingError::NotRead)
                }
            };
            let (mut memo, mut scratch) = (vec![], OperatorScratch::default());
            let mut context = OperatorContext {
                semantic: None,
                prepared_matchers: None,
                prepared_kinds: None,
                bindings: &[],
                symbols: (&mut symbols).into(),
                source: SourceView(b"ab"),
                paths: &[],
                arenas: &[],
                memo: &mut memo,
                scratch: &mut scratch,
            };
            let result = dag
                .demand_result(
                    root,
                    &read,
                    &crate::application::file_dag::execution::CommonOperatorEngine,
                    &mut context,
                )
                .unwrap();
            assert_eq!(result.kind(), *variant);
            assert_eq!(dag.executions[root.0], 1);
            let resolved = dag.relations.resolve(result.as_ref(), &read).unwrap();
            let raw = borrowed(&resolved);
            assert_eq!(kind(registered(*variant).bulk(raw).unwrap()), *variant);
            assert_eq!(raw.count(), count);
            assert_eq!(dag.relations.inspect(result.as_ref(),raw).unwrap(),crate::domain::table_model::collections::representations::types::PhysicalShape::Complete);
            let again = dag
                .demand_result(
                    root,
                    &read,
                    &crate::application::file_dag::execution::CommonOperatorEngine,
                    &mut context,
                )
                .unwrap();
            assert!(result.same_record(&again));
            assert_eq!(dag.executions[root.0], 1);
            dag.release_population();
            assert!(matches!(
                dag.relations.resolve(result.as_ref(), &read),
                Err(OperatingError::NotRead)
            ));
        }
    }
}
