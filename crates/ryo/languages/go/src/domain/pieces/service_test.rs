//! The `pieces` service's cases: the nodes and tokens found are kept in the order of the file, and a string keeps what it stands for.

use ryo::languages::NameText;

use super::service::{arrange, node, string, token, value_of};
use super::types::Pieces;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::grammar::types::ByteIndex;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;

/// What a row finds.
#[derive(Clone, Copy, Debug)]
enum Finding {
    /// Nodes over the same bytes, found outer first, and one that starts earlier found last.
    Nodes,
    /// A string with its value, and a token with none.
    Strings,
}

type Input = Finding;
/// The answer, as text.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "nodes_are_ordered_by_where_they_start_then_by_how_far_they_reach_then_by_when_they_were_found",
        ),
        word: Word::Succeeds,
        input: || Finding::Nodes,
        expect: Expect::Value(
            "FuncDecl 0..9, FuncType 0..8, FieldList 4..8, Field 5..7, Ident 5..7",
        ),
    },
    Case {
        name: CaseName("a_string_keeps_its_value_and_a_token_has_none"),
        word: Word::Succeeds,
        input: || Finding::Strings,
        expect: Expect::Value("String x, Add none"),
    },
];

fn answer(finding: Input) -> String {
    let mut pieces = Pieces::default();
    match finding {
        Finding::Nodes => {
            node(&mut pieces, Node::FieldList, ByteIndex(4), ByteIndex(8));
            node(&mut pieces, Node::Field, ByteIndex(5), ByteIndex(7));
            node(&mut pieces, Node::Ident, ByteIndex(5), ByteIndex(7));
            node(&mut pieces, Node::FuncType, ByteIndex(0), ByteIndex(8));
            node(&mut pieces, Node::FuncDecl, ByteIndex(0), ByteIndex(9));
            arrange(&mut pieces);
            pieces
                .nodes
                .iter()
                .map(|each| format!("{:?} {}..{}", each.kind, each.start.0, each.end.0))
                .collect::<Vec<_>>()
                .join(", ")
        }
        Finding::Strings => {
            string(&mut pieces, ByteIndex(0), ByteIndex(3), NameText("x"));
            token(&mut pieces, Token::Add, ByteIndex(3), ByteIndex(4));
            pieces
                .lexemes
                .iter()
                .map(|each| {
                    let value = value_of(&pieces, each).map_or("none", |text| text.0);
                    format!("{:?} {value}", each.kind)
                })
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}

fn subject(finding: Input) -> Value {
    Box::leak(answer(finding).into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
