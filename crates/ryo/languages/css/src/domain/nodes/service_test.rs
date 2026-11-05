//! The `nodes` service's cases: each node is in the standard's category.

use super::constants::STANDARD;
use super::service::category;
use super::types::{Category, CssNode};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// A node.
type Input = CssNode;
/// Its category.
type Value = Category;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_at_rule_is_a_rule"),
        word: Word::Succeeds,
        input: || CssNode::AtRule,
        expect: Expect::Value(Category::Rule),
    },
    Case {
        name: CaseName("a_simple_block_is_a_component_value"),
        word: Word::Succeeds,
        input: || CssNode::SimpleBlock,
        expect: Expect::Value(Category::ComponentValue),
    },
    Case {
        name: CaseName("a_declaration_is_its_own"),
        word: Word::Succeeds,
        input: || CssNode::Declaration,
        expect: Expect::Value(Category::Declaration),
    },
];

fn subject(node: Input) -> Value {
    category(node)
}

#[test]
fn succeeds() {
    assert!(STANDARD.contains("CSS Syntax Module Level 3"));
    run(CASES, Word::Succeeds, |node| {
        Ok::<Value, Failure>(subject(node))
    });
}
