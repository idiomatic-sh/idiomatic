//! All classes and the former pilot regressions use the same production executor.
pub(crate) mod fixtures;
use crate::application::file_dag::testing::{
    execute, execute_with_port, execute_with_semantic, same_rows,
};
use crate::domain::features::service::kind_of;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::service::registered;
use crate::outbound::operators::interface::OperatorsPort;
use fixtures::*;
use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::{
    plan_parts::types::ColumnIndex,
    plans::types::{Operation, OperationKind},
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Default)]
struct ContinuesAfterSetFailure {
    observed_denials: AtomicUsize,
}
impl OperatorsPort for ContinuesAfterSetFailure {
    fn operators(
        &self,
        call: ryo_operators::domain::contract::types::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::domain::contract::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    > {
        use ryo_operators::domain::contract::types::{
            OperatorAnswer, OperatorCall, SetInput, SetKind,
        };
        match call {
            OperatorCall::SetRow {
                input:
                    SetInput::Values {
                        kind: SetKind::Union,
                        row,
                        driver,
                        ..
                    },
            } => {
                let first = driver.emit(row);
                let second = driver.emit(row);
                let after_failure_distinct = driver.first_time(row);
                let after_failure_lookup = driver.contains_right(row);
                if first.is_ok()
                    && second.is_err()
                    && after_failure_distinct.is_err()
                    && after_failure_lookup.is_err()
                {
                    self.observed_denials.fetch_add(1, Ordering::Relaxed);
                }
                Ok(OperatorAnswer::Written)
            }
            other => crate::outbound::operators::factory::test_port().operators(other),
        }
    }
}

#[test]
fn set_delivery_failure_denies_later_callbacks_and_keeps_the_first_error() {
    let mut case = fixture(OperationKind::Union);
    let port = ContinuesAfterSetFailure::default();
    let observed = execute_with_port(
        &case.operation,
        &case.inputs,
        &case.bindings,
        SOURCE,
        &mut case.symbols,
        Some(1),
        None,
        Some(&port),
    )
    .expect("the prepared Union remains executable");
    assert_eq!(observed.stream_error, Some(OperatingError::WrongOperation));
    assert_eq!(observed.rows.len(), 1);
    assert_eq!(port.observed_denials.load(Ordering::Relaxed), 1);
}
#[test]
fn every_registered_class_runs_real_collect_and_stream_with_independent_expectations() {
    let semantic = KnownSyntax;
    assert_eq!(OperationKind::ALL.len(), 26);
    for kind in OperationKind::ALL {
        let mut case = fixture(*kind);
        assert_eq!(kind_of(&case.operation), *kind);
        let observed = execute_with_semantic(
            &case.operation,
            &case.inputs,
            &case.bindings,
            SOURCE,
            &mut case.symbols,
            None,
            if *kind == OperationKind::RenderSyntax {
                Some(&semantic)
            } else {
                None
            },
        )
        .unwrap();
        let collected = observed.collected.unwrap();
        assert_eq!(observed.stream_error, None, "{kind:?}");
        assert_eq!(observed.schema.unwrap(), collected.kinds);
        assert!(
            same_rows(&collected, &observed.rows),
            "ordered bag {kind:?}"
        );
        assert_expected(&case.expected, &observed.rows, &case.symbols);
    }
}

#[test]
fn unsupported_semantic_batch_publishes_no_prefix() {
    #[derive(Debug)]
    struct PartialSyntax;
    impl crate::domain::operating::types::SemanticLookup for PartialSyntax {
        fn render_batch(
            &self,
            _: &[RenderRequest],
            _: &[u8],
        ) -> Result<Vec<RenderOutcome>, OperatingError> {
            Ok(vec![
                RenderOutcome::Known(Arc::from("a")),
                RenderOutcome::Unsupported(ryo_relations::domain::terms::types::UnknownReason(24)),
            ])
        }
    }
    let mut case = fixture(OperationKind::RenderSyntax);
    crate::domain::table_model::service::push_row(
        &mut case.inputs[0],
        &[Value::Node(
            ryo_relations::domain::relations::service::node_id(
                ryo_relations::domain::relations::types::FileId(0),
                ryo_relations::domain::relations::types::Position(1),
            ),
        )],
    );
    let observed = execute_with_semantic(
        &case.operation,
        &case.inputs,
        &case.bindings,
        SOURCE,
        &mut case.symbols,
        None,
        Some(&PartialSyntax),
    )
    .unwrap();
    assert!(matches!(
        observed.collected,
        Err(OperatingError::SemanticIncomplete { .. })
    ));
    assert!(matches!(
        observed.stream_error,
        Some(OperatingError::SemanticIncomplete { .. })
    ));
    assert!(observed.rows.is_empty());
}

#[test]
fn all_registrations_refuse_a_foreign_variant() {
    for (at, kind) in OperationKind::ALL.iter().enumerate() {
        let foreign = fixture(OperationKind::ALL[(at + 1) % OperationKind::ALL.len()]);
        assert!(matches!(
            registered(*kind).prepare(&foreign.operation, &[], &[]),
            Err(OperatingError::WrongOperation)
        ));
    }
}
fn repeated(table: &Table, times: usize) -> Table {
    Table {
        kinds: table.kinds.clone(),
        cells: table.cells.repeat(times),
        count: ryo_values::domain::rows::types::RowCount(table.count.0 * times),
    }
}
#[test]
fn former_unary_pilots_keep_linear_visits_and_bounded_row_scratch() {
    for kind in [
        OperationKind::Select,
        OperationKind::Project,
        OperationKind::SourceMatches,
        OperationKind::SpanBounds,
    ] {
        let mut case = fixture(kind);
        if let Operation::Project { distinct, .. } = &mut case.operation {
            *distinct = ryo_values::domain::plan_parts::types::RepeatedRows::Kept;
        }
        let small = vec![repeated(&case.inputs[0], 32)];
        let large = vec![repeated(&case.inputs[0], 64)];
        let first = execute(
            &case.operation,
            &small,
            &[],
            SOURCE,
            &mut case.symbols,
            None,
        )
        .unwrap();
        let second = execute(
            &case.operation,
            &large,
            &[],
            SOURCE,
            &mut case.symbols,
            None,
        )
        .unwrap();
        assert!(first.visits[0] > 0);
        assert_eq!(second.visits[0], first.visits[0] * 2);
        assert_eq!(second.rows.len(), first.rows.len() * 2);
        assert_eq!(second.scratch_cells, first.scratch_cells);
        let empty = execute(
            &case.operation,
            &[repeated(&case.inputs[0], 0)],
            &[],
            SOURCE,
            &mut case.symbols,
            None,
        )
        .unwrap();
        assert!(empty.rows.is_empty());
        assert_eq!(empty.visits, vec![0]);
    }
}
#[test]
fn former_binary_pilots_keep_linear_visits_for_unique_keys() {
    for kind in [OperationKind::Join, OperationKind::Union] {
        let mut case = fixture(kind);
        if let Operation::Join { output, .. } = &mut case.operation {
            for column in output {
                column.column = ColumnIndex(0);
            }
        }
        let run = |count: i64, symbols: &mut Symbols| {
            let values = (0..count).collect::<Vec<_>>();
            execute(
                &case.operation,
                &[numbers(&values), numbers(&values)],
                &[],
                SOURCE,
                symbols,
                None,
            )
            .unwrap()
        };
        let first = run(32, &mut case.symbols);
        let second = run(64, &mut case.symbols);
        assert_eq!((first.rows.len(), second.rows.len()), (32, 64));
        for side in 0..2 {
            assert!(first.visits[side] > 0);
            assert_eq!(second.visits[side], first.visits[side] * 2);
        }
        let empty = run(0, &mut case.symbols);
        assert!(empty.rows.is_empty());
        assert_eq!(empty.visits, vec![0, 0]);
    }
}
#[test]
fn select_join_and_union_keep_independent_ordered_bags() {
    for kind in [
        OperationKind::Select,
        OperationKind::Join,
        OperationKind::Union,
    ] {
        let mut case = fixture(kind);
        let observed = execute(
            &case.operation,
            &case.inputs,
            &[],
            SOURCE,
            &mut case.symbols,
            None,
        )
        .unwrap();
        assert_expected(&case.expected, &observed.rows, &case.symbols);
    }
    let mut case = fixture(OperationKind::Join);
    case.inputs = vec![
        matrix(&[&[1, 10], &[1, 11], &[2, 12]]),
        matrix(&[&[1, 20], &[2, 21], &[1, 22]]),
    ];
    let observed = execute(
        &case.operation,
        &case.inputs,
        &[],
        SOURCE,
        &mut case.symbols,
        None,
    )
    .unwrap();
    assert_eq!(
        observed.rows,
        vec![
            vec![n(10), n(20)],
            vec![n(10), n(22)],
            vec![n(11), n(20)],
            vec![n(11), n(22)],
            vec![n(12), n(21)]
        ]
    );
}
#[test]
fn schema_failures_and_emitter_prefix_survive_the_common_path() {
    for kind in [
        OperationKind::Select,
        OperationKind::Project,
        OperationKind::SourceMatches,
        OperationKind::SpanBounds,
    ] {
        let mut case = fixture(kind);
        let mut invalid = case.operation.clone();
        match &mut invalid {
            Operation::Select { kept, .. } => kept.push(ColumnIndex(99)),
            Operation::Project { columns, .. } => columns[0] = ColumnIndex(99),
            Operation::SourceMatches { span, .. } | Operation::SpanBounds { span, .. } => {
                *span = ColumnIndex(99)
            }
            _ => unreachable!("four explicit fixtures"),
        }
        let observed =
            execute(&invalid, &case.inputs, &[], SOURCE, &mut case.symbols, None).unwrap();
        let schema = observed.schema.unwrap_err();
        assert_eq!(observed.collected.err(), Some(schema.clone()));
        assert_eq!(observed.stream_error, Some(schema));
        case.inputs = vec![repeated(&case.inputs[0], 2)];
        let observed = execute(
            &case.operation,
            &case.inputs,
            &[],
            SOURCE,
            &mut case.symbols,
            Some(1),
        )
        .unwrap();
        assert_eq!(observed.stream_error, Some(OperatingError::WrongOperation));
        assert_eq!(observed.rows.len(), 1);
        let collected = observed.collected.unwrap();
        assert_eq!(observed.rows[0], collected.cells[..collected.kinds.len()]);
    }
}
#[test]
fn binary_bad_shape_missing_input_and_sink_prefix_remain_observable() {
    for kind in [OperationKind::Join, OperationKind::Union] {
        let mut case = fixture(kind);
        let mut invalid = case.operation.clone();
        if let Operation::Join { left_on, .. } = &mut invalid {
            left_on[0] = ColumnIndex(99);
        } else {
            case.inputs[1] = fixture(OperationKind::SpanBounds).inputs.remove(0);
        }
        let observed =
            execute(&invalid, &case.inputs, &[], SOURCE, &mut case.symbols, None).unwrap();
        let schema = observed.schema.unwrap_err();
        assert_eq!(observed.collected.err(), Some(schema.clone()));
        assert_eq!(observed.stream_error, Some(schema));
        let mut case = fixture(kind);
        case.inputs.truncate(1);
        let observed = execute(
            &case.operation,
            &case.inputs,
            &[],
            SOURCE,
            &mut case.symbols,
            None,
        )
        .unwrap();
        let missing = OperatingError::MissingInput { position: 2 };
        assert_eq!(observed.schema, Err(missing.clone()));
        assert_eq!(observed.collected.err(), Some(missing.clone()));
        assert_eq!(observed.stream_error, Some(missing));
        let mut case = fixture(kind);
        let stopped = execute(
            &case.operation,
            &case.inputs,
            &[],
            SOURCE,
            &mut case.symbols,
            Some(1),
        )
        .unwrap();
        assert_eq!(stopped.stream_error, Some(OperatingError::WrongOperation));
        assert_eq!(stopped.rows.len(), 1);
        let collected = stopped.collected.unwrap();
        assert_eq!(stopped.rows[0], collected.cells[..collected.kinds.len()]);
    }
}
#[test]
fn conformance_detects_a_deliberately_dropped_or_reordered_result() {
    let mut case = fixture(OperationKind::Select);
    let observed = execute(
        &case.operation,
        &case.inputs,
        &[],
        SOURCE,
        &mut case.symbols,
        None,
    )
    .unwrap();
    let mut rows = observed.rows;
    let table = observed.collected.unwrap();
    assert!(same_rows(&table, &rows));
    rows.remove(0);
    assert!(!same_rows(&table, &rows));
    let mut rows = vec![vec![n(1)], vec![n(2)], vec![n(2)]];
    assert!(!same_rows(&table, &rows));
    rows.clear();
    assert!(!same_rows(&table, &rows));
}

#[test]
fn positive_row_contracts_keep_partial_input_distinction() {
    use crate::domain::coverage::types::{Lacking, Partial};
    use crate::domain::trust::{
        service::{blocked, needs_of, scoped_needs},
        types::Blocking,
    };
    use ryo_values::domain::plan_parts::types::{Input, StepId};
    use ryo_values::domain::plans::{
        service::{inputs_of, named},
        types::{Emit, Plan, PlanStep, StepScope},
    };
    use ryo_values::domain::stored_facts::types::PartialCause;
    for kind in [
        OperationKind::Select,
        OperationKind::Project,
        OperationKind::Join,
        OperationKind::Union,
        OperationKind::SourceMatches,
        OperationKind::SpanBounds,
    ] {
        let case = fixture(kind);
        let plan = Plan {
            parameters: vec![],
            steps: vec![PlanStep {
                operation: case.operation.clone(),
                scope: StepScope::PerFile,
            }],
            emit: Emit {
                step: StepId(0),
                subject: ColumnIndex(0),
                key: vec![],
                headings: vec![],
            },
            frontier: vec![],
            comparison_origins: vec![],
        };
        let needs = needs_of(&plan);
        assert_eq!(needs.len(), 1);
        assert!(needs[0].is_empty());
        let scoped = scoped_needs(&plan);
        for input in named(&inputs_of(&case.operation)) {
            if let Input::Relation(relation) = input {
                let partial = Partial {
                    relation: *relation,
                    cause: PartialCause("fixture partial".into()),
                    lacking: Lacking::Any,
                    at: None,
                };
                assert_eq!(blocked(&needs[0], &[partial]), Blocking::Free);
                assert!(scoped.reads.iter().any(|read| read.relation == *relation));
            }
        }
    }
}

use ryo_values::domain::rows::types::Table;
