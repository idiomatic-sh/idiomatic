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
        name: CaseName("a_css_file_is_read"),
        word: Word::Succeeds,
        input: || "styles/theme.css",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_ending_is_read_in_any_case"),
        word: Word::Succeeds,
        input: || "Site.CSS",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_preprocessor_file_is_declined"),
        word: Word::Succeeds,
        input: || "site.scss",
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a_file_of_another_language_is_declined"),
        word: Word::Succeeds,
        input: || "index.html",
        expect: Expect::Value(false),
    },
];

fn subject(path: Input) -> Value {
    reads(languages::SourcePath(path)) == languages::Recognition::Reads
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        Ok::<Value, Failure>(subject(input))
    });
}
