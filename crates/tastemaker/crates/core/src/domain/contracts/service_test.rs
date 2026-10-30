//! The `contracts` service's cases, on values alone: how a contract is compared, from what its rendered content answered.

use super::service::{facts, shape};
use super::types::{DataRow, RowKey, RowNode, RowPlace, Shape, Syntax, SyntaxKind, WrittenValue};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::definitions::types::Exactness;

/// The relations a content answered, and whether its template is exact.
type Input = (&'static [Syntax], Exactness);
type Value = Shape;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_content_with_keys_is_compared_as_data"),
        word: Word::Succeeds,
        input: || (&[Syntax::Scopes, Syntax::Entries], Exactness::Exact),
        expect: Expect::Value(Shape::Data),
    },
    Case {
        name: CaseName("an_exact_content_with_prose_is_compared_in_order"),
        word: Word::Succeeds,
        input: || (&[Syntax::Scopes, Syntax::Prose], Exactness::Exact),
        expect: Expect::Value(Shape::Document),
    },
    Case {
        name: CaseName("an_open_content_with_prose_is_compared_by_its_declarations"),
        word: Word::Succeeds,
        input: || (&[Syntax::Scopes, Syntax::Prose], Exactness::Contains),
        expect: Expect::Value(Shape::Code),
    },
    Case {
        name: CaseName("a_content_that_answered_nothing_is_blind"),
        word: Word::Succeeds,
        input: || (&[], Exactness::Contains),
        expect: Expect::Value(Shape::Blind),
    },
];

/// One row of each relation `answered`.
fn rows(answered: &[Syntax]) -> Vec<DataRow> {
    answered
        .iter()
        .map(|syntax| DataRow {
            syntax: *syntax,
            id: RowNode("0".into()),
            owner: RowNode("0".into()),
            key: RowKey::default(),
            kind: SyntaxKind::default(),
            text: WrittenValue::default(),
            span: RowPlace::Unplaced,
        })
        .collect()
}

fn subject((answered, exact): Input) -> Value {
    shape(&facts(&rows(answered)), exact)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
