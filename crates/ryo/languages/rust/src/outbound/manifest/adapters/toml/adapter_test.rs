//! The `toml` adapter of the manifest port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::ManifestTomlAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::paths::types::FileText;
use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::manifest::fixtures::contract::{check, shown};
use crate::outbound::manifest::interface::ManifestPort as _;

/// The text of a manifest.
type Input = &'static str;
/// The manifest, shown.
type Value = &'static str;
type Failure = ManifestError;

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_workspace_is_read_with_its_members_and_exclusions"),
        word: Word::Succeeds,
        input: || "[workspace]\nmembers = [\"crates/*\"]\nexclude = [\"crates/old\"]\n",
        expect: Expect::Value("workspace members crates/* exclude crates/old"),
    },
    Case {
        name: CaseName("a_dependency_is_read_with_its_kind_platform_and_path"),
        word: Word::Succeeds,
        input: || "[package]\nname = \"app\"\n[target.'cfg(unix)'.dev-dependencies]\nhelper = { path = \"../helper\", package = \"real\", optional = true }\n",
        expect: Expect::Value(
            "package app dependency helper Dev cfg(unix) ../helper real optional",
        ),
    },
    Case {
        name: CaseName("a_value_taken_from_the_workspace_is_noted"),
        word: Word::Succeeds,
        input: || "[package]\nname = \"app\"\nversion.workspace = true\n",
        expect: Expect::Value("package app inherits"),
    },
    Case {
        name: CaseName("a_dependency_taken_from_the_workspace_is_noted"),
        word: Word::Succeeds,
        input: || "[package]\nname = \"app\"\n[dependencies]\nshared.workspace = true\n",
        expect: Expect::Value("package app dependency shared Normal    inherits"),
    },
    Case {
        name: CaseName("text_that_is_not_toml_is_refused"),
        word: Word::Refuses,
        input: || "[package\nbroken",
        expect: Expect::Refusal(|error| {
            selects(
                matches!(error, ManifestError::Unreadable { reason } if &*reason.0 == "unclosed table, expected `]`"),
            )
        }),
    },
    Case {
        name: CaseName("a_manifest_with_neither_a_package_nor_a_workspace_is_refused"),
        word: Word::Refuses,
        input: || "[dependencies]\n",
        expect: Expect::Refusal(|error| {
            selects(
                matches!(error, ManifestError::Unreadable { reason } if &*reason.0 == "no `package` or `workspace` section"),
            )
        }),
    },
];

fn subject(text: Input) -> Result<Value, Failure> {
    ManifestTomlAdapter
        .read(&FileText(text.into()))
        .map(|manifest| shown(&manifest))
}

#[test]
fn succeeds() {
    check(&ManifestTomlAdapter);
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
