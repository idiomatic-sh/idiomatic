//! Source-location row serialization keeps its coordinate columns distinct.

use super::super::interface::RelationsItem as _;
use super::item::{SourceLocation, SourceLocationRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Integer, NodeId, Offset, Span};

type Input = ();
type Value = [Bits; 5];
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("coordinates_have_distinct_columns"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(3), Bits(5), Bits(30_064_771_083), Bits(13), Bits(17)]),
}];

fn subject((): Input) -> Result<Value, Failure> {
    Ok(SourceLocation::words(SourceLocationRow {
        id: NodeId(3),
        file: FileId(5),
        span: Span {
            start: Offset(7),
            end: Offset(11),
        },
        start_line: Integer(13),
        end_line: Integer(17),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
