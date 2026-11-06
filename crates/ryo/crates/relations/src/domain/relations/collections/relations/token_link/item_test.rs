//! Links use existing token IDs and one ordinary typed relation row.
use super::super::interface::RelationsItem as _;
use super::item::{TokenLink, TokenLinkRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;

type Input = ();
type Value = [Bits; 5];
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("token_edges_preserve_ids_and_source_token_span"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(3), Bits(9), Bits(17), Bits(7), Bits(47_244_640_269)]),
}];

fn subject(_: Input) -> Result<Value, Failure> {
    Ok(TokenLink::words(TokenLinkRow {
        source: NodeId(3),
        target: NodeId(9),
        link_kind: Symbol(17),
        file: FileId(7),
        span: Span {
            start: Offset(11),
            end: Offset(13),
        },
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
