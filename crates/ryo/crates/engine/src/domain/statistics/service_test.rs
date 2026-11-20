//! The `statistics` service's cases: bytes that hold no statistics are absent, never an error, whatever they are.

use super::service::decoded;
use super::types::StatisticsBytes;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};

/// The bytes a row decodes.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// No bytes: a rule set that ships no statistics.
    NoBytes,
    /// Bytes that are no statistics.
    OtherBytes,
}

/// Whether bytes held statistics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Held {
    /// They read as statistics.
    Statistics,
    /// They are absent.
    Absent,
}

type Input = Scenario;
type Value = Held;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_bytes_are_no_statistics"),
        word: Word::Succeeds,
        input: || Scenario::NoBytes,
        expect: Expect::Value(Held::Absent),
    },
    Case {
        name: CaseName("bytes_that_do_not_read_are_absent_never_an_error"),
        word: Word::Succeeds,
        input: || Scenario::OtherBytes,
        expect: Expect::Value(Held::Absent),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let kinds = KindVocabulary {
        language: LanguageName("walking".into()),
        kinds: Vec::new(),
    };
    let bytes: &[u8] = match scenario {
        Scenario::NoBytes => &[],
        Scenario::OtherBytes => b"not statistics",
    };
    Ok(decoded(StatisticsBytes(bytes), &kinds).map_or(Held::Absent, |_| Held::Statistics))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
