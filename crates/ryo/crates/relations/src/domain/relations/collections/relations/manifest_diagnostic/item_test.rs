//! Actual primary file identity, reported span and diagnostic categories occupy independent columns.
use super::super::interface::RelationsItem as _;
use super::item::{ManifestDiagnostic, ManifestDiagnosticRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    relations::types::{Bits, FileId, Offset, Span},
    terms::types::Symbol,
};

type Input = ();
type Value = [Bits; 7];
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("real_file_packed_span_and_diagnostic_categories_keep_column_order"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([
        Bits(3),
        Bits(21_474_836_489),
        Bits(11),
        Bits(13),
        Bits(15),
        Bits(17),
        Bits(19),
    ]),
}];

fn subject(_: Input) -> Result<Value, Failure> {
    Ok(ManifestDiagnostic::words(ManifestDiagnosticRow {
        file: FileId(3),
        span: Span {
            start: Offset(5),
            end: Offset(9),
        },
        location_kind: Symbol(11),
        phase: Symbol(13),
        schema: Symbol(15),
        code: Symbol(17),
        message: Symbol(19),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
