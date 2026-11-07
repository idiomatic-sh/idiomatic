//! The typed cells' cases: a row is shown as the test writes it.

use super::super::rendered::shown;
use super::{Cell, name, node, span, unknown};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Relation;

/// What the case writes.
type Input = fn() -> String;
/// How the row is shown.
type Value = &'static str;
type Failure = ();

fn a_row_with_an_unknown_value() -> String {
    shown(
        Relation::Function,
        &[
            Cell::Node(node(3).0),
            unknown("wildcard-import").cell(),
            Cell::File("f.rs".to_owned()),
            Cell::Span(span((1, 5), (1, 6)).from, span((1, 5), (1, 6)).to),
        ],
    )
}

fn a_row() -> String {
    shown(
        Relation::Function,
        &[
            Cell::Node(node(3).0),
            name("x").cell(),
            Cell::File("f.rs".to_owned()),
            Cell::Span(span((1, 5), (1, 6)).from, span((1, 5), (1, 6)).to),
        ],
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("diagnostic_node_identifiers_are_readable_rust_literals"),
        word: Word::Succeeds,
        input: || || Cell::Node(2_147_483_648).to_string(),
        expect: Expect::Value("node(2_147_483_648)"),
    },
    Case {
        name: CaseName("a_row_is_shown_as_it_is_written"),
        word: Word::Succeeds,
        input: || a_row,
        expect: Expect::Value(
            "FunctionExpectedRow { id: node(3), name: name(\"x\"), file: file(\"f.rs\"), span: span((1, 5), (1, 6)) }",
        ),
    },
    Case {
        name: CaseName("a_value_the_language_cannot_resolve_is_shown_with_its_reason"),
        word: Word::Succeeds,
        input: || a_row_with_an_unknown_value,
        expect: Expect::Value(
            "FunctionExpectedRow { id: node(3), name: unknown(\"wildcard-import\"), file: file(\"f.rs\"), span: span((1, 5), (1, 6)) }",
        ),
    },
];

fn subject(row: Input) -> Result<Value, Failure> {
    Ok(Box::leak(row().into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
