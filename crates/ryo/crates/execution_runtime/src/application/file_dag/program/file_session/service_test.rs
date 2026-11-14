//! The planner and the file session agree on which numbers are stored: the recogniser classifies an ordered existence
//! only over a number the session hands over as stored columns, which the runtime's number admission requires.
use super::service::source_rows;
use super::types::SourceRows;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::self_join_recogniser::fixtures::proves_stored_number;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::service::{all, kinds};
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, FileId, Relation};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::borrow::Cow;

/// The relations whose first number column is checked.
type Input = &'static [Relation];

/// How many relations have a number column, and those where the recogniser and the session disagree.
#[derive(Debug, PartialEq)]
struct Agreement {
    numbered: usize,
    disagreeing: Cow<'static, [Relation]>,
}

type Value = Agreement;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_recogniser_proves_a_number_exactly_when_the_session_hands_it_over_stored"),
    word: Word::Succeeds,
    input: all,
    expect: Expect::Value(Agreement {
        numbered: 25,
        disagreeing: Cow::Borrowed(&[]),
    }),
}];

/// Hands each relation over as stored columns and asks the recogniser about its first number.
fn subject(relations: Input) -> Result<Value, Failure> {
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut numbered = 0_usize;
    let mut disagreeing = Vec::new();
    for relation in relations {
        let schema = kinds(*relation);
        let Some(number) = schema.iter().position(|kind| *kind == CellKind::Number) else {
            continue;
        };
        numbered += 1;
        let read = |_: Relation| -> Result<SourceRows<'_>, OperatingError> {
            Ok(SourceRows::Stored(ColumnsView {
                kinds: schema,
                columns: &[],
            }))
        };
        let stored = source_rows(&read, &elements, FileId(0), *relation)
            .ok()
            .is_some_and(|rows| matches!(rows, Rows::Stored(_)));
        let column =
            ColumnIndex(u32::try_from(number).map_err(|_| OperatingError::WrongOperation)?);
        if proves_stored_number(*relation, column) != stored {
            disagreeing.push(*relation);
        }
    }
    Ok(Agreement {
        numbered,
        disagreeing: disagreeing.into(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
