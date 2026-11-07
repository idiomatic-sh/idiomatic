//! Public canonical error/delivery parity and independent ordered-bag controls.
use super::super::super::types::OperatingError;
use super::super::conformance::test::{compare_delivery, repeated, same_rows};
use crate::application::file_dag::acceptance::operator_contract_test::fixtures::*;
use crate::application::file_dag::acceptance::operator_contract_test::fixtures::values::n;
use crate::application::file_dag::acceptance::operator_contract_test::fixtures::values::matrix;
use crate::application::file_dag::acceptance::operator_contract_test::fixtures::expectations::{SOURCE, assert_expected};
use ryo_values::domain::{
    plan_parts::types::ColumnIndex,
    plans::types::Operation,
    query_operators::types::OperationKind,
};
#[test]
fn select_join_and_union_keep_independent_ordered_bags() {
    for kind in [
        OperationKind::Select,
        OperationKind::Join,
        OperationKind::Union,
    ] {
        let mut case = fixture(kind);
        let observed = compare_delivery(
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
    let observed = compare_delivery(
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
            compare_delivery(&invalid, &case.inputs, &[], SOURCE, &mut case.symbols, None).unwrap();
        let schema = observed.schema.unwrap_err();
        assert_eq!(observed.collected.err(), Some(schema.clone()));
        assert_eq!(observed.stream_error, Some(schema));
        case.inputs = vec![repeated(&case.inputs[0], 2)];
        let observed = compare_delivery(
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
            compare_delivery(&invalid, &case.inputs, &[], SOURCE, &mut case.symbols, None).unwrap();
        let schema = observed.schema.unwrap_err();
        assert_eq!(observed.collected.err(), Some(schema.clone()));
        assert_eq!(observed.stream_error, Some(schema));
        let case = fixture(kind);
        let missing = crate::domain::operation_schema::service::schema_of(
            &case.operation,
            &[crate::domain::operation_schema::types::InputShape {
                kinds: &case.inputs[0].kinds,
                tree: None,
            }],
            &[],
        );
        assert_eq!(missing, Err(OperatingError::MissingInput { position: 2 }));
        // Runtime RHS refusal (with a valid successful LHS) is covered by the
        // public provider_test requests; an absent raw input slice is not a plan.
        let mut case = fixture(kind);
        let stopped = compare_delivery(
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
    let observed = compare_delivery(
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
    use ryo_values::domain::plan_parts::types::StepScope;
    use ryo_values::domain::plan_parts::types::{Input, StepId};
    use ryo_values::domain::plans::{
        service::{inputs_of, named},
        types::{Emit, Plan, PlanStep},
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
            observation_inputs: Vec::new(),
            observations: Vec::new(),
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
