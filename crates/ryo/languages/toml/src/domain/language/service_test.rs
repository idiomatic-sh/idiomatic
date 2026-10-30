//! The `language` service's cases.

use super::service::reads;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::languages;

/// A path.
type Input = &'static str;
/// Whether the language reads the file.
type Value = bool;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_toml_file_is_read"),
        word: Word::Succeeds,
        input: || "crates/core/Cargo.toml",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_lock_file_written_in_toml_is_read"),
        word: Word::Succeeds,
        input: || "crates/core/Cargo.lock",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_file_of_another_language_is_declined"),
        word: Word::Succeeds,
        input: || "src/lib.rs",
        expect: Expect::Value(false),
    },
];

fn subject(path: Input) -> Value {
    reads(languages::SourcePath(path)) == languages::Recognition::Reads
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
