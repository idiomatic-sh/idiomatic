//! The `expression_fields` service's cases: the fields the children of an expression fill, as its typed node says them, in the
//! order they are written.

use super::service::expression_child_fields;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ruff_python_ast::{AnyNodeRef, Stmt};

/// A source whose first statement is an expression asked for the fields of its children.
type Input = &'static str;
/// The names of the fields, in order, joined by a space.
type Value = &'static str;
/// A source the parser refuses, or with no statement.
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_conditional_expression_s_children_fill_body_test_and_orelse"),
        word: Word::Succeeds,
        input: || "yes if asked else no\n",
        expect: Expect::Value("body test orelse"),
    },
    Case {
        name: CaseName("a_comparison_s_first_operand_fills_left_and_the_rest_comparators"),
        word: Word::Succeeds,
        input: || "low < middle < high\n",
        expect: Expect::Value("left comparators comparators"),
    },
];

fn subject(source: Input) -> Result<Value, Failure> {
    let parsed = ruff_python_parser::parse_module(source).map_err(|_refused| ())?;
    let Some(Stmt::Expr(statement)) = parsed.syntax().body.first() else {
        return Err(());
    };
    let mut fields = Vec::new();
    expression_child_fields(AnyNodeRef::from(statement.value.as_ref()), &mut fields);
    let names: Vec<&'static str> = fields.iter().map(|each| each.field.into()).collect();
    Ok(Box::leak(names.join(" ").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
