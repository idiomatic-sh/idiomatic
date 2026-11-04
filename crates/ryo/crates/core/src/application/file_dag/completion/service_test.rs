use super::*;
use crate::domain::coverage::types::{FilePartial, Lacking};
use ryo_relations::domain::relations::types::{Relation, Value};
use ryo_values::domain::{
    plan_parts::types::ColumnIndex, rows::types::RowCount, stored_facts::types::PartialCause,
};
use std::cell::Cell;
fn partial() -> Partial {
    Partial {
        relation: Relation::SourceLocation,
        cause: PartialCause("fixture missing facts".into()),
        lacking: Lacking::Any,
        at: Some(FileId(0)),
    }
}
fn anti() -> Operation {
    Operation::Antijoin {
        left: Input::Relation(Relation::SourceLocation),
        right: Input::Relation(Relation::SourceLocation),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
    }
}
fn needs() -> Vec<RelationRead> {
    vec![RelationRead {
        relation: Relation::SourceLocation,
        kinds: vec![],
    }]
}
#[test]
fn file_blocks_before_run_but_project_preserves_evaluation_error_timing() {
    let partials = [partial()];
    let calls = Cell::new(0);
    let file = CompletionPolicy::file(RuleIndex(3), FileId(0), &partials);
    let result = file
        .evaluate(&anti(), &needs(), &[CellKind::File], || {
            calls.set(calls.get() + 1);
            Err(OperatingError::NotRead)
        })
        .unwrap();
    assert!(matches!(
        result,
        NodeCompletion::Unavailable(UnavailableCompletion {
            reason: UnavailableReason::AbsenceBlockedByPartial,
            ..
        })
    ));
    assert_eq!(calls.get(), 0);
    let project_partials = Partials {
        entries: vec![FilePartial {
            file: Some(FileId(0)),
            partial: partial(),
        }],
        unavailable_files: vec![],
    };
    let project = CompletionPolicy::project(RuleIndex(3), &project_partials);
    let result = project.evaluate(&anti(), &needs(), &[CellKind::File], || {
        calls.set(calls.get() + 1);
        Err(OperatingError::NotRead)
    });
    assert!(matches!(result, Err(OperatingError::NotRead)));
    assert_eq!(calls.get(), 1);
}
#[test]
fn project_filters_after_success_and_retains_partial_coverage() {
    let partials = Partials {
        entries: vec![FilePartial {
            file: Some(FileId(0)),
            partial: partial(),
        }],
        unavailable_files: vec![],
    };
    let policy = CompletionPolicy::project(RuleIndex(4), &partials);
    let result = policy
        .evaluate(&anti(), &needs(), &[CellKind::File], || {
            Ok(Table {
                kinds: vec![CellKind::File],
                cells: vec![Value::File(FileId(0)), Value::File(FileId(1))],
                count: RowCount(2),
            })
        })
        .unwrap();
    let NodeCompletion::Available { rows, coverage } = result else {
        panic!("successful collection")
    };
    assert_eq!(
        coverage,
        CoverageState::Partial {
            files: vec![Some(FileId(0))]
        }
    );
    let rows = rows.into_table();
    assert_eq!(rows.count, RowCount(1));
    assert_eq!(rows.cells, [Value::File(FileId(1))]);
}
#[test]
fn missing_frontier_is_unavailable_with_exact_scope_and_input_not_complete_empty() {
    let partials = Partials::default();
    let policy = CompletionPolicy::project(RuleIndex(5), &partials);
    let input = Input::Relation(Relation::SourceLocation);
    assert!(
        matches!(policy.missing_frontier(input), NodeCompletion::Unavailable(UnavailableCompletion {
        scope: ScopeIdentity::Project { rule: RuleIndex(5) }, reason: UnavailableReason::MissingFrontier { input: actual }
    }) if actual == input)
    );
}
