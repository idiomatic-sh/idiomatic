//! False is an explicit typed flag, distinct from an absent relation capability.
use super::super::interface::RelationsItem as _;
use super::item::{CompilerFeature, CompilerFeatureRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Flag};
use crate::domain::terms::types::Symbol;

type Input = Flag;
type Value = [Bits; 3];
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_raised_feature_flag_is_one_beside_its_file_and_feature"),
        word: Word::Succeeds,
        input: || Flag::Raised,
        expect: Expect::Value([Bits(7), Bits(19), Bits(1)]),
    },
    Case {
        name: CaseName("a_lowered_feature_flag_is_zero_beside_its_file_and_feature"),
        word: Word::Succeeds,
        input: || Flag::Lowered,
        expect: Expect::Value([Bits(7), Bits(19), Bits(0)]),
    },
];

fn subject(enabled: Input) -> Result<Value, Failure> {
    Ok(CompilerFeature::words(CompilerFeatureRow {
        file: FileId(7),
        feature: Symbol(19),
        enabled,
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
