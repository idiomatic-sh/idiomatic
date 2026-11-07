//! The `pip` adapter of the manifest reading port: the port's contract rows run through it, and the shapes of path it claims.

use super::adapter::ManifestReadingPipAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::manifest_reading::errors::ManifestReadingError;
use crate::outbound::manifest_reading::fixtures::contract::{Sample, check};
use crate::outbound::manifest_reading::interface::ManifestReadingPort as _;
use crate::outbound::manifest_reading::types::Claim;
use ryo::languages::FilePath;

/// The path of a file.
type Input = &'static str;
/// Whether the adapter claims it.
type Value = Claim;
type Failure = ManifestReadingError;

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("requirements_dot_txt_is_claimed_at_any_depth"),
        word: Word::Succeeds,
        input: || "services/api/requirements.txt",
        expect: Expect::Value(Claim::Reads),
    },
    Case {
        name: CaseName("a_requirements_file_with_a_suffix_is_claimed"),
        word: Word::Succeeds,
        input: || "requirements-dev.txt",
        expect: Expect::Value(Claim::Reads),
    },
    Case {
        name: CaseName("a_text_file_in_a_requirements_directory_is_claimed"),
        word: Word::Succeeds,
        input: || "requirements/base.txt",
        expect: Expect::Value(Claim::Reads),
    },
    Case {
        name: CaseName("a_text_file_elsewhere_is_not_claimed"),
        word: Word::Succeeds,
        input: || "docs/notes.txt",
        expect: Expect::Value(Claim::Skips),
    },
    Case {
        name: CaseName("a_non_text_file_in_a_requirements_directory_is_not_claimed"),
        word: Word::Succeeds,
        input: || "requirements/base.in",
        expect: Expect::Value(Claim::Skips),
    },
];

fn subject(path: Input) -> Value {
    ManifestReadingPipAdapter::new(Vec::new()).claims(&FilePath(path.into()))
}

#[test]
fn succeeds() {
    check(
        ManifestReadingPipAdapter::new,
        Sample {
            path: "requirements.txt",
            readable: "requests>=2\n",
            broken: "-r missing.txt\n",
            others: &[],
        },
    );
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
