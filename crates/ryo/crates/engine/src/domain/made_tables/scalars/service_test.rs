//! The null word has no ordering and only the true flag satisfies a predicate.
use super::types::CollationText;
use super::{constants::MISSING_CELL, service, types::PackedScalar};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Flag;
use ryo_values::domain::table_words::types::PackedSlice;

type Input = bool;
type Value = bool;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("true_flag"),
        word: Word::Succeeds,
        input: || true,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("false_flag"),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(false),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    let flag = service::flag(if input { Flag::Raised } else { Flag::Lowered });
    let missing = PackedScalar {
        kind: CellKind::Number,
        word: MISSING_CELL,
    };
    assert_eq!(
        service::compare(missing, flag, |_| PackedSlice(b""), |_| PackedSlice(b"")),
        None
    );
    assert_eq!(
        service::spelling(flag, |_| PackedSlice(b""), |_| PackedSlice(b""))
            .0
            .as_ref(),
        input.to_string().as_bytes()
    );
    assert!(service::natural_order(CollationText("column 9"), CollationText("column 10")).is_lt());
    assert!(service::natural_order(CollationText("column 02"), CollationText("column 1")).is_lt());
    assert!(!service::natural_order(CollationText("x 2"), CollationText("x2")).is_lt());
    assert!(service::natural_order(CollationText("λ"), CollationText("λ2")).is_lt());
    Ok(service::truth(flag) == Flag::Raised)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
