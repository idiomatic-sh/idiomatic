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
        name: CaseName("a_go_file_is_read"),
        word: Word::Succeeds,
        input: || "src/main.go",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_test_file_is_read"),
        word: Word::Succeeds,
        input: || "main_test.go",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_file_of_another_language_is_declined"),
        word: Word::Succeeds,
        input: || "README.md",
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a_module_file_is_read_for_its_requirements"),
        word: Word::Succeeds,
        input: || "go.mod",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_module_file_in_a_directory_is_read"),
        word: Word::Succeeds,
        input: || "tools/go.mod",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_workspace_file_is_declined"),
        word: Word::Succeeds,
        input: || "go.work",
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a_file_whose_name_only_ends_in_the_module_file_is_declined"),
        word: Word::Succeeds,
        input: || "notgo.mod",
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a_file_whose_name_only_contains_go_is_declined"),
        word: Word::Succeeds,
        input: || "notes.go.md",
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
