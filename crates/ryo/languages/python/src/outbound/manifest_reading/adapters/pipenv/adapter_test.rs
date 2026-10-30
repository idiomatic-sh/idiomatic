//! The `pipenv` adapter of the manifest reading port: the port's contract rows run through it, and the paths it claims.

use super::adapter::ManifestReadingPipenvAdapter;
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
        name: CaseName("a_pipfile_is_claimed_at_any_depth"),
        word: Word::Succeeds,
        input: || "apps/web/Pipfile",
        expect: Expect::Value(Claim::Reads),
    },
    Case {
        name: CaseName("a_lock_file_is_not_claimed"),
        word: Word::Succeeds,
        input: || "Pipfile.lock",
        expect: Expect::Value(Claim::Skips),
    },
];

fn subject(path: Input) -> Value {
    ManifestReadingPipenvAdapter.claims(&FilePath(path.into()))
}

#[test]
fn succeeds() {
    check(
        |_| ManifestReadingPipenvAdapter,
        Sample {
            path: "app/Pipfile",
            readable: "[packages]\nrequests = \"*\"\n",
            broken: "[packages\nrequests = ",
            others: &[],
        },
    );
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
