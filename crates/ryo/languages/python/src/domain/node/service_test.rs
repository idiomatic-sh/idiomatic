//! The `node` service's cases: the field a token fills under the node that holds it, and what a walked file answers for an id.

use super::service::{id_of, node_at, token_field, typed_nodes, written};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::node::types::Numbered;
use crate::domain::tokens::types::Token;
use crate::fixtures::syntax_files::with_file;
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::{AnyNodeRef, Expr, Stmt};
use ryo::languages::Position;

/// What a case asks.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// The field of a token of this kind and name under the first expression of this source.
    Token(&'static str, TokenKind, Token),
    /// Whether every node of this source's walked file answers its own id, and what the last name of the file is written as.
    Ids(&'static str),
}

type Input = Asked;
/// The name of the field, or the text found.
type Value = &'static str;
/// A source that holds no expression statement, or that is not walked.
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_name_s_identifier_fills_id"),
        word: Word::Succeeds,
        input: || Asked::Token("x\n", TokenKind::Identifier, Token::Name),
        expect: Expect::Value("id"),
    },
    Case {
        name: CaseName("an_operation_s_operator_fills_op"),
        word: Word::Succeeds,
        input: || Asked::Token("(a + b)\n", TokenKind::Plus, Token::Plus),
        expect: Expect::Value("op"),
    },
    Case {
        name: CaseName("a_bracket_of_an_operation_fills_no_field"),
        word: Word::Succeeds,
        input: || Asked::Token("(a + b)\n", TokenKind::Lpar, Token::Lpar),
        expect: Expect::Value("none"),
    },
    Case {
        name: CaseName("every_node_answers_its_id_and_its_text"),
        word: Word::Succeeds,
        input: || Asked::Ids("first = 1\nsecond = first\n"),
        expect: Expect::Value("first"),
    },
];

/// The first expression statement's expression of a parsed source.
fn first_expression(body: &[Stmt]) -> Option<&Expr> {
    body.iter().find_map(|statement| {
        if let Stmt::Expr(expression) = statement {
            Some(expression.value.as_ref())
        } else {
            None
        }
    })
}

fn subject(asked: Input) -> Result<Value, Failure> {
    match asked {
        Asked::Token(source, kind, named) => {
            let parsed = ruff_python_parser::parse_module(source).map_err(|_refused| ())?;
            let expression = first_expression(&parsed.syntax().body).ok_or(())?;
            Ok(token_field(AnyNodeRef::from(expression), kind, named).into())
        }
        Asked::Ids(source) => {
            let mut found = String::new();
            with_file(source, &mut |file| {
                for Numbered { id, node } in typed_nodes(file) {
                    if id_of(node) != Some(id) || node_at(file, id).is_none() {
                        found.push_str("a node does not answer its id; ");
                    }
                }
                let last = typed_nodes(file)
                    .filter(|each| matches!(each.node, AnyNodeRef::ExprName(_)))
                    .last()
                    .map_or(Position(0), |each| each.id);
                found.push_str(written(file, last).0);
            });
            Ok(Box::leak(found.into_boxed_str()))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
