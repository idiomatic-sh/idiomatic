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
        name: CaseName("a_readme_is_read"),
        word: Word::Succeeds,
        input: || "crates/core/README.md",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_long_ending_is_read"),
        word: Word::Succeeds,
        input: || "docs/guide.markdown",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_text_file_is_declined"),
        word: Word::Succeeds,
        input: || "README.txt",
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("mdx_is_another_language"),
        word: Word::Succeeds,
        input: || "docs/page.mdx",
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
