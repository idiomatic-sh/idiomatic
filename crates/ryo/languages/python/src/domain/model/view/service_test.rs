//! The cases of the model's per-file view. Track P1's placeholder of the file track P2 writes: the placeholder builds nothing.

use super::service::built;
use super::types::{Built, ModelPart, SemanticModel};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::languages::NodeTable;

type Input = ModelPart;
/// Whether the part asked for is kept.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_placeholder_keeps_the_part_asked_for"),
    word: Word::Succeeds,
    input: || ModelPart::Lexical,
    expect: Expect::Value("asked"),
}];

fn subject(part: Input) -> Result<Value, Failure> {
    let (mut model, mut parts) = (SemanticModel::default(), Built::default());
    built(&mut model, &mut parts, part, &NodeTable::default(), &[]);
    Ok(if parts.asked == Some(part) && model.asked == [part] {
        "asked"
    } else {
        "not asked"
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
