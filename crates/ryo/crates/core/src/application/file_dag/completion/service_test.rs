use super::*;
use crate::domain::coverage::types::{FilePartial, Lacking};
use ryo_relations::domain::relations::types::{CellKind, Relation, Value};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input},
    rows::types::RowCount,
    stored_facts::types::PartialCause,
};
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
fn evaluation_refusal_cannot_become_complete_when_publication_inputs_are_complete() {
    let evaluation = [partial()];
    let policy = CompletionPolicy::file_inputs(RuleIndex(3), FileId(0), &evaluation, &[]);
    assert!(!policy.allows_region(&needs()));
    assert!(policy.observation_ready(&needs()));
    assert_eq!(
        policy.blocked_coverage(),
        CoverageState::Partial {
            files: vec![Some(FileId(0))]
        }
    );
}

#[test]
fn project_filter_preserves_other_files_and_exact_physical_coverage() {
    let partials = Partials {
        entries: vec![FilePartial {
            file: Some(FileId(0)),
            partial: partial(),
        }],
        unavailable_files: vec![],
    };
    let policy = CompletionPolicy::project_inputs(RuleIndex(4), &partials, &partials);
    let mut table = Table {
        kinds: vec![CellKind::File],
        cells: vec![Value::File(FileId(0)), Value::File(FileId(1))],
        count: RowCount(2),
    };
    policy.filter_rows(&anti(), &needs(), &mut table);
    assert_eq!(table.cells, vec![Value::File(FileId(1))]);
    assert_eq!(table.count, RowCount(1));
    assert_eq!(
        policy.coverage(&needs()),
        CoverageState::Partial {
            files: vec![Some(FileId(0))]
        }
    );
}
