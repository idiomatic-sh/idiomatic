//! What the receipt rows accept and refuse, one row of `CASES` for each input.
use super::rows;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::receipts::fixtures::samples::observation;

type Input = &'static str;
type Value = usize;
type Failure = ();

/// A row the writer wrote is read back; a field the writer never writes is refused.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a written row is read back as one document"),
        word: Word::Succeeds,
        input: || "written",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a row with a field nobody writes is refused"),
        word: Word::Refuses,
        input: || "surprise",
        expect: Expect::Error(()),
    },
];

/// Reads one written row, or one with an extra field.
fn subject(kind: Input) -> Result<Value, Failure> {
    let bytes = crate::outbound::receipts::adapters::local::observation::schemas::row_bytes(&observation())
        .map_err(|_refused| ())?;
    let mut line = String::from_utf8(bytes).map_err(|_refused| ())?;
    if kind == "surprise" {
        line = line.replacen('{', "{\"surprise\":1,", 1);
    }
    rows(&line).map(|documents| documents.len()).map_err(|_refused| ())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
