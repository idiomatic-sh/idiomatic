//! A key points to an original occurrence in the same source file.

use super::super::interface::RelationsItem as _;
use super::item::{ConstantKey, ConstantKeyRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{FileId, NodeId, Offset, Span};

type Input = ();
type Value = [u64; 4];
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("constant keys retain occurrence and representative identities"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([2, 5, 3, (11_u64 << 32) | 17]),
}];

fn subject(_: Input) -> Result<Value, Failure> {
    Ok(ConstantKey::words(ConstantKeyRow {
        id: NodeId(2),
        key: NodeId(5),
        file: FileId(3),
        span: Span {
            start: Offset(11),
            end: Offset(17),
        },
    })
    .map(|word| word.0))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
