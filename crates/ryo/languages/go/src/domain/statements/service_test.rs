//! The `statements` service's cases: the nodes `go/ast` makes of a switch, a select, a type switch, a loop, a receive, a label and a jump. Each row runs the service over the nodes of a source's tree and says the nodes it finds, in order.

use super::service::{branch, clauses, labeled, receive, repetition, type_switch};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::grammar::service::form;
use crate::domain::grammar::types::{Form, Index};
use crate::domain::nodes::types::GoNode;
use crate::fixtures::reading::found_by;

/// A source.
type Input = &'static str;
/// The kinds of the nodes the service finds over the nodes of its tree, in order.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_switch_holds_its_clauses_in_a_block"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\n\tswitch x {\n\tcase 1:\n\tdefault:\n\t}\n}\n",
        expect: Expect::Value("switch-stmt block-stmt"),
    },
    Case {
        name: CaseName("a_select_holds_its_clauses_in_a_block_and_its_receive_is_an_expr_stmt"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\n\tselect {\n\tcase <-c:\n\tdefault:\n\t}\n}\n",
        expect: Expect::Value("select-stmt block-stmt expr-stmt"),
    },
    Case {
        name: CaseName("a_type_switch_has_the_assignment_and_the_assertion"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\n\tswitch t := x.(type) {\n\t}\n}\n",
        expect: Expect::Value("type-switch-stmt block-stmt assign-stmt type-assert-expr"),
    },
    Case {
        name: CaseName("a_type_switch_with_no_name_has_an_expression_statement"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\n\tswitch x.(type) {\n\t}\n}\n",
        expect: Expect::Value("type-switch-stmt block-stmt expr-stmt type-assert-expr"),
    },
    Case {
        name: CaseName("a_loop_is_a_for_stmt_and_a_range_is_a_range_stmt"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\n\tfor i := 0; i < 1; i++ {\n\t}\n\tfor range x {\n\t}\n}\n",
        expect: Expect::Value("for-stmt range-stmt"),
    },
    Case {
        name: CaseName("a_receive_that_assigns_is_an_assign_stmt"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\n\tselect {\n\tcase v := <-c:\n\tcase <-d:\n\t}\n}\n",
        expect: Expect::Value("select-stmt block-stmt assign-stmt expr-stmt"),
    },
    Case {
        name: CaseName("a_label_with_no_statement_has_an_empty_statement"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\nL:\n}\n",
        expect: Expect::Value("labeled-stmt empty-stmt"),
    },
    Case {
        name: CaseName("a_label_holds_the_statement_after_it"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\nL:\n\tx++\n}\n",
        expect: Expect::Value("labeled-stmt"),
    },
    Case {
        name: CaseName("a_jump_is_a_branch_stmt"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\n\tfor {\n\t\tbreak\n\t\tcontinue\n\t}\n\tgoto L\n}\n",
        expect: Expect::Value("for-stmt branch-stmt branch-stmt branch-stmt"),
    },
];

/// The nodes the service finds over each node of the tree of `source`, as the walk asks them of it: the service of a form and no other.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the forms this service finds nodes for; every other form is another service's"
)]
fn subject(source: Input) -> Value {
    Box::leak(
        found_by(source, |tree, pieces| {
            for at in (0..tree.nodes.len()).map(Index) {
                match form(tree, at) {
                    Form::Switch => clauses(tree, pieces, GoNode::SwitchStmt, at),
                    Form::Select => clauses(tree, pieces, GoNode::SelectStmt, at),
                    Form::TypeSwitch => type_switch(tree, pieces, at),
                    Form::Loop => repetition(tree, pieces, at),
                    Form::Receive => receive(tree, pieces, at),
                    Form::Labeled => labeled(tree, pieces, at),
                    Form::Branch(jump) => branch(tree, pieces, at, jump),
                    _ => {}
                }
            }
        })
        .into_boxed_str(),
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
