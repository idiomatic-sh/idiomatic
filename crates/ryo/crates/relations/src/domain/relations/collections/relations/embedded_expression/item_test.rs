//! Every embedded-expression column has an independently chosen value.
use super::super::interface::RelationsItem as _;
use super::item::{EmbeddedExpression, EmbeddedExpressionRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;
/// No parser is needed to verify the row encoding.
type Input = ();
/// Root, host, context, mapping, file, and the two span offsets.
type Value = [Bits; 6];
type Failure = ();
/// Distinct words catch column reorderings.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("embedded provenance row layout"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([
        Bits(3),
        Bits(5),
        Bits(7),
        Bits(11),
        Bits(13),
        Bits(73_014_444_051),
    ]),
}];
fn subject(_input: Input) -> Result<Value, Failure> {
    Ok(EmbeddedExpression::words(EmbeddedExpressionRow {
        root: NodeId(3),
        host: NodeId(5),
        context: Symbol(7),
        mapping: Symbol(11),
        file: FileId(13),
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
