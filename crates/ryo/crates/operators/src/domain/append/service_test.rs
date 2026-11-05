//! Append preserves order and uses the engine's scratch storage.
use super::service::append;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

type Input = ();
type Value = [Cell; 3];
type Failure = ();
const ROW: [Cell; 2] = [Cell::Number(Integer(2)), Cell::Number(Integer(2))];
const SUFFIX: [Cell; 1] = [Cell::Number(Integer(5))];
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("append_preserves_duplicate_row_values_and_suffix_order"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([ROW[0], ROW[1], SUFFIX[0]]),
}];
fn subject((): Input) -> Result<Value, Failure> {
    let mut output = Vec::with_capacity(8);
    output.push(Cell::Number(Integer(99)));
    let original = output.as_ptr();
    append(&ROW, &SUFFIX, &mut output);
    assert_eq!(
        output.as_ptr(),
        original,
        "scratch allocation must be reused"
    );
    Ok(output
        .try_into()
        .expect("two row cells and one suffix cell"))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
