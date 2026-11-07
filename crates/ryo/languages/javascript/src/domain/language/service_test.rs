//! The `language` service's cases.

use super::service::{goal, reads};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::types::Goal;
use ryo::languages;

/// A path.
type Input = &'static str;
/// Whether the language reads the file, and how.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_javascript_file_is_read_with_module_syntax_detected"),
        word: Word::Succeeds,
        input: || "src/main.js",
        expect: Expect::Value("detected"),
    },
    Case {
        name: CaseName("a_jsx_file_is_read_with_module_syntax_detected"),
        word: Word::Succeeds,
        input: || "src/view.jsx",
        expect: Expect::Value("detected"),
    },
    Case {
        name: CaseName("an_mjs_file_is_read_as_a_module"),
        word: Word::Succeeds,
        input: || "src/main.mjs",
        expect: Expect::Value("module"),
    },
    Case {
        name: CaseName("a_cjs_file_is_read_as_a_script"),
        word: Word::Succeeds,
        input: || "src/main.cjs",
        expect: Expect::Value("script"),
    },
    Case {
        name: CaseName("a_file_of_another_language_is_declined"),
        word: Word::Succeeds,
        input: || "README.md",
        expect: Expect::Value("declined"),
    },
];

fn subject(path: Input) -> Value {
    let path = languages::SourcePath(path);
    match (reads(path), goal(path)) {
        (languages::Recognition::Declines, Goal::Module | Goal::Script | Goal::Detected) => {
            "declined"
        }
        (languages::Recognition::Reads, Goal::Module) => "module",
        (languages::Recognition::Reads, Goal::Script) => "script",
        (languages::Recognition::Reads, Goal::Detected) => "detected",
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
