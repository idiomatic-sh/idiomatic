//! The `node_fields` service's cases: the fields the children of a node fill, as its typed node says them, in the order they are
//! written.

use super::service::child_fields;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::labels::types::Field;
use crate::domain::node::types::Placed;
use ruff_python_ast::AnyNodeRef;

/// A source whose first statement is asked for the fields of its children.
type Input = &'static str;
/// The names of the fields, in order, joined by a space.
type Value = &'static str;
/// A source the parser refuses, or with no statement.
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_function_s_children_fill_its_fields_in_written_order"),
        word: Word::Succeeds,
        input: || "@decorated\ndef function(parameter) -> int:\n    pass\n",
        expect: Expect::Value("decorator-list name args returns body"),
    },
    Case {
        name: CaseName("a_loop_s_else_fills_orelse"),
        word: Word::Succeeds,
        input: || "for item in items:\n    pass\nelse:\n    pass\n",
        expect: Expect::Value("target iter body orelse"),
    },
];

fn subject(source: Input) -> Result<Value, Failure> {
    let parsed = ruff_python_parser::parse_module(source).map_err(|_refused| ())?;
    let statement = parsed.syntax().body.first().ok_or(())?;
    let mut fields = Vec::new();
    child_fields(
        Placed {
            node: AnyNodeRef::from(statement),
            field: Field::Body,
            parent: None,
        },
        &mut fields,
    );
    let names: Vec<&'static str> = fields.iter().map(|each| each.field.into()).collect();
    Ok(Box::leak(names.join(" ").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
