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
        name: CaseName("a_module_is_read"),
        word: Word::Succeeds,
        input: || "ci/gates.py",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_stub_is_read"),
        word: Word::Succeeds,
        input: || "typings/app.pyi",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_module_run_without_a_console_is_read"),
        word: Word::Succeeds,
        input: || "tools/app.pyw",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_file_of_another_language_is_declined"),
        word: Word::Succeeds,
        input: || "pyproject.toml",
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

#[test]
fn explicit_primary_roles_distinguish_configuration_from_language_source() {
    use super::service::primary_role;
    for path in ["pyproject.toml", "pkg/pyproject.toml", "Pipfile"] {
        assert_eq!(
            primary_role(
                languages::SourcePath(path),
                &[
                    languages::ManifestName("Pipfile"),
                    languages::ManifestName("pyproject.toml")
                ]
            ),
            languages::PrimaryFileRole::Configuration
        );
    }
    for path in [
        "invalid.py",
        "input.txt",
        "module.pyi",
        "pyproject.toml.py",
        "other.toml",
        "poetry.lock",
    ] {
        assert_eq!(
            primary_role(
                languages::SourcePath(path),
                &[languages::ManifestName("pyproject.toml")]
            ),
            languages::PrimaryFileRole::Source
        );
    }
}
