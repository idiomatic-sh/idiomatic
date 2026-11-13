//! Every literal parse column has an independently chosen encoding.
use super::super::interface::RelationsItem as _;
use super::item::{LiteralExpressionParse, LiteralExpressionParseRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;
/// No parser is needed to verify the row encoding.
type Input = ();
/// Carrier, dialect, outcome, file, and two physical span offsets.
type Value = [Bits; 5];
/// The row conversion is infallible.
type Failure = ();
/// Distinct words catch column reorderings.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("literal parse row layout"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(3), Bits(5), Bits(7), Bits(11), Bits(73_014_444_051)]),
}];
/// Encode one independently specified row.
fn subject(_input: Input) -> Result<Value, Failure> {
    Ok(LiteralExpressionParse::words(LiteralExpressionParseRow {
        id: NodeId(3),
        dialect: Symbol(5),
        outcome: Symbol(7),
        file: FileId(11),
        span: Span {
            start: Offset(17),
            end: Offset(19),
        },
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
