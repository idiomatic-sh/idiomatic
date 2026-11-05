//! The standard distinctions Oxc stores in discriminants rather than distinct node types.

use super::node_kind;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use oxc_allocator::Allocator;
use oxc_ast::{AstKind, ast::Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;

#[derive(Clone, Copy, Debug)]
enum Given {
    One(&'static str),
    Every,
}
#[derive(Debug, PartialEq, Eq)]
enum Seen {
    Kind(Option<Node>),
    Accounted(bool),
}
type Input = Given;
type Value = Seen;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("function_declaration_keeps_its_identity"),
        word: Word::Succeeds,
        input: || Given::One("function run() {}"),
        expect: Expect::Value(Seen::Kind(Some(Node::FunctionDeclaration))),
    },
    Case {
        name: CaseName("ambient_function_is_not_a_function_body"),
        word: Word::Succeeds,
        input: || Given::One("declare function run(): void;"),
        expect: Expect::Value(Seen::Kind(Some(Node::TSDeclareFunction))),
    },
    Case {
        name: CaseName("class_declaration_keeps_its_identity"),
        word: Word::Succeeds,
        input: || Given::One("class Counter {}"),
        expect: Expect::Value(Seen::Kind(Some(Node::ClassDeclaration))),
    },
    Case {
        name: CaseName("every_kind_of_the_library_is_mapped_or_dropped"),
        word: Word::Succeeds,
        input: || Given::Every,
        expect: Expect::Value(Seen::Accounted(true)),
    },
];

/// Every lexer kind comes from Oxc's own inventory, including kinds not emitted as source tokens.
fn library_kinds() -> Vec<oxc_parser::Kind> {
    oxc_parser::Kind::VARIANTS.to_vec()
}
/// Parser-control markers have no token in the standard tree; comments are supplied separately.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "Only explicitly justified parser controls may be dropped; any other unmapped kind makes accounted fail"
)]
fn dropped(kind: oxc_parser::Kind) -> Option<&'static str> {
    match kind {
        oxc_parser::Kind::Eof => Some("end of input is not a source token"),
        oxc_parser::Kind::Skip => {
            Some("comments and whitespace are written from their source ranges")
        }
        oxc_parser::Kind::Undetermined => {
            Some("unrecognized input is an error, never a mapped token")
        }
        oxc_parser::Kind::HashbangComment => {
            Some("hashbang is retained as an extra, outside AST_TOKEN_TYPES")
        }
        _ => None,
    }
}
/// A lexer kind is mapped or has a documented reason not to produce a standard token.
fn accounted(kind: oxc_parser::Kind) -> bool {
    match super::token_kind(kind) {
        Ok(token) => {
            crate::domain::syntax::service::kind_of(crate::domain::syntax::service::kind_bits(
                crate::domain::syntax::types::Kind::Token(token),
            )) == crate::domain::syntax::types::Kind::Token(token)
        }
        Err(unrecognized) => dropped(unrecognized).is_some_and(|reason| !reason.is_empty()),
    }
}
fn subject(given: Input) -> Value {
    let Given::One(source) = given else {
        return Seen::Accounted(library_kinds().into_iter().all(accounted));
    };
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    Seen::Kind(match parsed.program.body.first() {
        Some(Statement::FunctionDeclaration(node)) => node_kind(AstKind::Function(node)),
        Some(Statement::ClassDeclaration(node)) => node_kind(AstKind::Class(node)),
        _ => None,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
