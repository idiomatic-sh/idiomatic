//! The real file identity and interned path occupy separate columns.
use super::super::interface::RelationsItem as _;
use super::item::{ModuleIdentity, ModuleIdentityRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    relations::types::{Bits, FileId},
    terms::types::Symbol,
};

type Input = ();
type Value = [Bits; 5];
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("row_words_preserve_column_order"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(3), Bits(5), Bits(7), Bits(9), Bits(11)]),
}];

fn subject(_: Input) -> Result<Value, Failure> {
    Ok(ModuleIdentity::words(ModuleIdentityRow {
        file: FileId(3),
        path: Symbol(5),
        module_name: Symbol(7),
        package_name: Symbol(9),
        package_root_name: Symbol(11),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
