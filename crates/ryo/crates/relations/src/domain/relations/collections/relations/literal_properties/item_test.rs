//! The row keeps three independent predicates and the original source identity.

use super::super::interface::RelationsItem as _;
use super::item::{LiteralProperties, LiteralPropertiesRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Flag, NodeId, Offset, Span};

type Input = ();
type Value = [Bits; 7];
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("literal property flags stay independent of the literal category"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([
        Bits(3),
        Bits(17),
        Bits(1),
        Bits(0),
        Bits(0),
        Bits(7),
        Bits(47_244_640_269),
    ]),
}];

fn subject(_: Input) -> Result<Value, Failure> {
    Ok(LiteralProperties::words(LiteralPropertiesRow {
        id: NodeId(3),
        kind: crate::domain::terms::types::Symbol(17),
        zero: Flag::Raised,
        empty: Flag::Lowered,
        boolean: Flag::Lowered,
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
