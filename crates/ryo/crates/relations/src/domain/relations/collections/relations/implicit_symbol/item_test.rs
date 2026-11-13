//! Independent distinct values guard the catalog layout without invented source positions.
use super::super::interface::RelationsItem as _;
use super::item::{ImplicitSymbol, ImplicitSymbolRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Integer, NodeId};
use crate::domain::terms::types::Symbol;
type Input = ();
type Value = [Bits; 6];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("catalog name version and file occupy distinct cells"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(5), Bits(7), Bits(13), Bits(3), Bits(11), Bits(17)]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(ImplicitSymbol::words(ImplicitSymbolRow {
        name: Symbol(5),
        kind: Symbol(7),
        scope: NodeId(13),
        since_major: Integer(3),
        since_minor: Integer(11),
        file: FileId(17),
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
