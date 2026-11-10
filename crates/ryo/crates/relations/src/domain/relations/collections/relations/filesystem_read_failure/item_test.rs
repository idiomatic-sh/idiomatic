//! Typed failure rows preserve the primary identity, derived detail and real file origin.
use super::super::interface::RelationsItem as _;
use super::item::{FilesystemReadFailure, FilesystemReadFailureRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Offset, Span};
use crate::domain::terms::types::Symbol;
type Input = ();
type Value = [Bits; 3];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_primary_file_detail_and_origin_have_distinct_columns"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(3), Bits(5), Bits(0)]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(FilesystemReadFailure::words(FilesystemReadFailureRow {
        path: FileId(3),
        detail: Symbol(5),
        span: Span {
            start: Offset(0),
            end: Offset(0),
        },
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
