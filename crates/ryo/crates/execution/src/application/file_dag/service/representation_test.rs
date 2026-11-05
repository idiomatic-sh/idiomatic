//! Actual DAG demands retain each bulk representation through the mandatory entry.
use super::*;
use crate::application::file_dag::interface::DagExecution as _;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::collections::representations::{
    publication::PublishedRelation,
    service::{borrowed, kind, registered},
    types::RepresentationKind,
};
use crate::domain::table_model::types::{SelectedRows, StoredSelection};
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::{
    elements::{
        service::{enter_node, leave_node, table_builder, token},
        types::Elements,
    },
    symbols::types::{ByteSlice, Symbols},
};
use ryo_relations::domain::relations::{
    service::{kinds, to_bits},
    types::{ColumnsView, FactColumn, FileId, Integer, KindBits, Offset},
};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, RepeatedRows, StepId},
    plans::types::{Emit as PlanEmit, Operation, Plan, PlanStep, StepScope},
};

fn plan(relation: Relation) -> Plan {
    Plan {
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
fn dag_preserves_all_bulk_routes_with_and_without_optional_retention() {
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
    let mut words = ryo_relations::domain::relations::types::Words::default();
    ryo_values::domain::table_words::service::encode_table(&computed, &[], &mut words);
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
            RepresentationKind::Compact => (Relation::LanguageTarget, Rows::Compact(&compact), 2),
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
                    crate::outbound::derived::factory::test_port(),
                )
                .unwrap();
            let graph = Arc::new(crate::domain::file_program::service::compile_prepared(
                &plans, &operators,
            ));
            let root = graph.cuts[0][0].expect("internal representation producer");
            assert_ne!(
                graph.boundaries[0].first().map(|(_, terminal)| *terminal),
                Some(root),
                "the residual terminal is a distinct consumer"
            );
            let scope = crate::application::file_dag::testing::FileScope::new(
                graph,
                crate::application::file_dag::execution::CommonOperatorEngine::fixture_ref(),
                |_| true,
                &[relation],
                &[],
            )
            .unwrap();
            let dag = &scope.execution;
            dag.borrow_mut().limit = budget;
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
                .demand(root, &read, scope.scope.executor(), &mut context)
                .unwrap();
            assert_eq!(result.kind(), *variant);
            assert_eq!(
                dag.borrow().executions[root.0],
                1,
                "ordinary source is executed once"
            );
            let resolved = scope.scope.owner().resolve(result.as_ref(), &read).unwrap();
            let raw = borrowed(&resolved);
            assert_eq!(kind(registered(*variant).bulk(raw).unwrap()), *variant);
            assert_eq!(raw.count(), count);
            assert_eq!(scope.scope.owner().inspect(result.as_ref(),raw).unwrap(),crate::domain::table_model::collections::representations::types::PhysicalShape::Complete);
            let again = dag
                .demand(root, &read, scope.scope.executor(), &mut context)
                .unwrap();
            assert!(result.same_record(&again));
            assert_eq!(dag.borrow().executions[root.0], 1);
            dag.borrow_mut().release_population();
            assert!(matches!(
                scope.scope.owner().resolve(result.as_ref(), &read),
                Err(OperatingError::NotRead)
            ));
        }
    }
}

#[test]
fn supplied_compact_is_published_once_then_retired_or_latched_refused() {
    use crate::application::file_dag::types::ProvidedInput;
    use ryo_relations::domain::relations::types::Words;
    use ryo_values::domain::stored_facts::types::KeptWords;
    use ryo_values::domain::table_words::{
        service as codec,
        types::{DecodeMode, WordSlice},
    };

    let relation = Relation::LanguageTarget;
    let input = Input::Relation(relation);
    let mut plan = plan(relation);
    plan.steps.truncate(1);
    plan.emit.step = StepId(0);
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
    let root = graph.boundaries[0][0].1;
    let complete = Table {
        kinds: ryo_relations::domain::relations::service::kinds(relation).to_vec(),
        cells: vec![
            Value::File(FileId(1)),
            Value::Number(Integer(7)),
            Value::Number(Integer(9)),
            Value::File(FileId(2)),
            Value::Number(Integer(8)),
            Value::Number(Integer(10)),
        ],
        count: RowCount(2),
    };
    let short = Table {
        kinds: vec![CellKind::File, CellKind::Number],
        cells: vec![Value::File(FileId(3)), Value::Number(Integer(7))],
        count: RowCount(1),
    };
    for refused in [false, true] {
        let mut words = Words::default();
        codec::encode_table(&complete, &[], &mut words);
        if refused {
            codec::encode_table(&short, &[], &mut words);
        }
        let (compact, names) = codec::admit_compact(
            Arc::new(KeptWords(codec::pack(WordSlice(&words.0)))),
            &mut Vec::new(),
            if refused {
                DecodeMode::Permissive
            } else {
                DecodeMode::Complete
            },
        )
        .unwrap();
        assert!(names.is_empty());
        assert_eq!(codec::compact_rectangular(&compact), !refused);
        let scope = crate::application::file_dag::testing::FileScope::new(
            Arc::clone(&graph),
            crate::application::file_dag::execution::CommonOperatorEngine::fixture_ref(),
            |_| true,
            &[relation],
            &[],
        )
        .unwrap();
        let dag = &scope.execution;
        dag.borrow_mut().provided.insert(
            root,
            ProvidedInput::Compact {
                input,
                table: Arc::new(compact),
            },
        );
        let (mut symbols, mut scratch, mut memo) =
            (Symbols::default(), OperatorScratch::default(), vec![]);
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
        let unread = |_| Err(OperatingError::NotRead);
        dag.begin_rule(0).unwrap();
        if refused {
            for _ in 0..2 {
                assert!(matches!(
                    dag.demand(root, &unread, scope.scope.executor(), &mut context),
                    Err(OperatingError::ShapeMismatch)
                ));
            }
            assert!(matches!(
                dag.borrow().provided.get(&root),
                Some(ProvidedInput::Refused { .. })
            ));
        } else {
            let first = dag
                .demand(root, &unread, scope.scope.executor(), &mut context)
                .unwrap();
            assert_eq!(first.kind(), RepresentationKind::Compact);
            let second = dag
                .demand(root, &unread, scope.scope.executor(), &mut context)
                .unwrap();
            assert!(first.same_record(&second));
            assert!(matches!(
                dag.borrow().provided.get(&root),
                Some(ProvidedInput::Live { .. })
            ));
            assert_eq!(
                dag.borrow().executions[root.0],
                0,
                "supplied source is published, not executed"
            );
            dag.borrow_mut().finish_rule(0);
            assert!(matches!(
                dag.borrow().provided.get(&root),
                Some(ProvidedInput::Retired { .. })
            ));
            dag.begin_rule(0)
                .expect("a deliberate retry has a required rule scope");
            let after_retirement = dag.demand(root, &unread, scope.scope.executor(), &mut context);
            assert!(
                matches!(
                    after_retirement,
                    Err(OperatingError::RetiredFrontier { .. })
                ),
                "retired supplied input must latch its refusal: {after_retirement:?}"
            );
        }
    }
}
