//! Lexical scopes exclude ordinary value expressions.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::TypeScriptNode;
use crate::domain::syntax::types::Kind;
type Input = Kind;
type Value = super::types::Scope;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_file_is_a_scope"),
        word: Word::Succeeds,
        input: || Kind::Node(TypeScriptNode::Program),
        expect: Expect::Value(super::types::Scope::Opens),
    },
    Case {
        name: CaseName("a_block_owns_lexical_bindings"),
        word: Word::Succeeds,
        input: || Kind::Node(TypeScriptNode::BlockStatement),
        expect: Expect::Value(super::types::Scope::Opens),
    },
    Case {
        name: CaseName("a_call_is_not_a_scope"),
        word: Word::Succeeds,
        input: || Kind::Node(TypeScriptNode::CallExpression),
        expect: Expect::Value(super::types::Scope::Continues),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        Ok(super::service::opens(input))
    });
}
