use super::*;
use crate::domain::coverage::types::{FilePartial, Lacking};
use ryo_relations::domain::relations::types::{Relation, Value};
use ryo_values::domain::{
    plan_parts::types::ColumnIndex, rows::types::RowCount, stored_facts::types::PartialCause,
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
fn file_partial_is_node_local_without_running_semantics_and_project_preserves_failure() {
    let engine = crate::application::file_dag::execution::CommonOperatorEngine::fixture();
    let scope = crate::application::file_dag::testing::semantic_scope(&engine).unwrap();
    let partials = [partial()];
    let file = CompletionPolicy::file(RuleIndex(3), FileId(0), &partials);
    let NodeAdmission::Blocked { rows, coverage } = file
        .admit(&needs(), &Ok(vec![CellKind::File]), scope.owner())
        .unwrap()
    else {
        panic!("absence-sensitive node must be blocked");
    };
    assert_eq!(
        coverage,
        CoverageState::Partial {
            files: vec![Some(FileId(0))]
        }
    );
    let read = |_| Err(OperatingError::NotRead);
    let publication = rows.into_payload();
    let rows = scope.owner().resolve(publication.as_ref(), &read).unwrap();
    assert_eq!(
        crate::domain::table_model::service::count_of(&rows),
        RowCount(0)
    );
    let project_partials = Partials {
        entries: vec![FilePartial {
            file: Some(FileId(0)),
            partial: partial(),
        }],
        unavailable_files: vec![],
    };
    let project = CompletionPolicy::project(RuleIndex(3), &project_partials);
    let result = project.finish(
        &anti(),
        &needs(),
        &[CellKind::File],
        Err(OperatingError::NotRead),
        scope.owner(),
        &read,
    );
    assert!(matches!(
        result,
        NodeCompletion::Failed(OperatingError::NotRead)
    ));
}
#[test]
fn project_filters_after_success_and_retains_partial_coverage() {
    let engine = crate::application::file_dag::execution::CommonOperatorEngine::fixture();
    let scope = crate::application::file_dag::testing::semantic_scope(&engine).unwrap();
    let partials = Partials {
        entries: vec![FilePartial {
            file: Some(FileId(0)),
            partial: partial(),
        }],
        unavailable_files: vec![],
    };
    let policy = CompletionPolicy::project(RuleIndex(4), &partials);
    let publication = scope
        .owner()
        .computed(Table {
            kinds: vec![CellKind::File],
            cells: vec![Value::File(FileId(0)), Value::File(FileId(1))],
            count: RowCount(2),
        })
        .unwrap();
    let read = |_| Err(OperatingError::NotRead);
    let NodeCompletion::Available { rows, coverage } = policy.finish(
        &anti(),
        &needs(),
        &[CellKind::File],
        Ok(publication),
        scope.owner(),
        &read,
    ) else {
        panic!("successful collection");
    };
    assert_eq!(
        coverage,
        CoverageState::Partial {
            files: vec![Some(FileId(0))]
        }
    );
    let publication = rows.into_payload();
    let rows = scope.owner().resolve(publication.as_ref(), &read).unwrap();
    let mut table = Table::default();
    crate::domain::table_model::service::materialize(&rows, &mut table, &mut Vec::new());
    assert_eq!(table.count, RowCount(1));
    assert_eq!(table.cells, [Value::File(FileId(1))]);
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
