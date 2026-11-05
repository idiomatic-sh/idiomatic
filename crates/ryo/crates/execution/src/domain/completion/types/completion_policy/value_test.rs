use super::value::*;
use crate::domain::completion::types::CoverageState;
use crate::domain::coverage::types::{FilePartial, Lacking};
use crate::domain::coverage::types::{Partial, Partials, RuleIndex};
use crate::domain::trust::types::RelationRead;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::{CellKind, Relation, Value as Cell};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput},
    rows::types::RowCount,
    stored_facts::types::PartialCause,
};
use ryo_values::domain::{plans::types::Operation, rows::types::Table};
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
        left: PlanInput::Relation(Relation::SourceLocation),
        right: PlanInput::Relation(Relation::SourceLocation),
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
        cells: vec![Cell::File(FileId(0)), Cell::File(FileId(1))],
        count: RowCount(2),
    };
    policy.filter_rows(&anti(), &needs(), &mut table);
    assert_eq!(table.cells, vec![Cell::File(FileId(1))]);
    assert_eq!(table.count, RowCount(1));
    assert_eq!(
        policy.coverage(&needs()),
        CoverageState::Partial {
            files: vec![Some(FileId(0))]
        }
    );
}

use ryo_values::cases::{Case, CaseName, Expect, Word, run};
type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("evaluation_refusal_preserves_partial_coverage"),
        word: Word::Succeeds,
        input: || evaluation_refusal_cannot_become_complete_when_publication_inputs_are_complete,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("project_filter_preserves_physical_coverage"),
        word: Word::Succeeds,
        input: || project_filter_preserves_other_files_and_exact_physical_coverage,
        expect: Expect::Value(()),
    },
];
fn subject(case: Input) -> Result<Value, Failure> {
    case();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
