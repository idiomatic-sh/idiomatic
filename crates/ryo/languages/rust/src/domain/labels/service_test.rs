//! The `labels` service's cases, on values alone: what an element is, and the role its kind alone gives it.

use super::service::{category, kind_role, token_role};
use super::types::Category;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::RustNode;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// What a row of the cases asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// What an element of this kind is.
    Category(Kind),
    /// The role a node of this kind has by its kind alone.
    Kind(Kind),
    /// The role a token of the second kind has under a parent of the first.
    Token(Kind, Kind),
}

/// What a row of the cases asks of the service.
type Input = Scenario;
/// The name of the category or of the role.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_binary_expression_is_an_expression"),
        word: Word::Succeeds,
        input: || Scenario::Category(Kind::Node(RustNode::BinExpr)),
        expect: Expect::Value("expression"),
    },
    Case {
        name: CaseName("a_function_is_an_item"),
        word: Word::Succeeds,
        input: || Scenario::Category(Kind::Node(RustNode::Fn)),
        expect: Expect::Value("item"),
    },
    Case {
        name: CaseName("a_let_statement_is_a_statement"),
        word: Word::Succeeds,
        input: || Scenario::Category(Kind::Node(RustNode::LetStmt)),
        expect: Expect::Value("statement"),
    },
    Case {
        name: CaseName("a_name_is_the_name_of_its_parent"),
        word: Word::Succeeds,
        input: || Scenario::Kind(Kind::Node(RustNode::Name)),
        expect: Expect::Value("name"),
    },
    Case {
        name: CaseName("a_path_segment_is_a_segment"),
        word: Word::Succeeds,
        input: || Scenario::Kind(Kind::Node(RustNode::PathSegment)),
        expect: Expect::Value("segment"),
    },
    Case {
        name: CaseName("a_function_has_no_role_by_its_kind"),
        word: Word::Succeeds,
        input: || Scenario::Kind(Kind::Node(RustNode::Fn)),
        expect: Expect::Value("none"),
    },
    Case {
        name: CaseName("the_plus_of_a_binary_expression_is_its_operator"),
        word: Word::Succeeds,
        input: || Scenario::Token(Kind::Node(RustNode::BinExpr), Kind::Token(Token::Plus)),
        expect: Expect::Value("op"),
    },
    Case {
        name: CaseName("a_name_token_of_a_function_has_no_role"),
        word: Word::Succeeds,
        input: || Scenario::Token(Kind::Node(RustNode::Fn), Kind::Token(Token::FnKw)),
        expect: Expect::Value("none"),
    },
];

/// The category as text.
const fn shown(what: Category) -> &'static str {
    match what {
        Category::Expression => "expression",
        Category::Item => "item",
        Category::Statement => "statement",
        Category::Pattern | Category::Type | Category::Other => "other",
    }
}

fn subject(given: Input) -> Value {
    match given {
        Scenario::Category(element) => shown(category(element)),
        Scenario::Kind(element) => kind_role(element).into(),
        Scenario::Token(parent, element) => token_role(parent, element).into(),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
