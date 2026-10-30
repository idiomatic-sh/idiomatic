//! Every project parser accepts an empty package and rejects truncated JSON.

use super::super::interface::ManifestReadingPort;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::packages::service::directory;
use crate::domain::packages::types::{Catalog, Catalogs, Declared, RepositoryPath};
use ryo::languages::{FilePath, ManifestFile, ManifestText};
use std::collections::BTreeMap;

/// The text of the manifest a row hands the adapter.
type Input = &'static str;
/// What the adapter declares for it.
type Value = Declared;
/// A refused manifest: the one way an adapter fails.
type Failure = ();
/// The rows every manifest reading adapter answers alike: an empty package, and truncated JSON.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("empty_package_has_no_edges"),
        word: Word::Succeeds,
        input: || "{}",
        expect: Expect::Value(Declared {
            name: None,
            members: Vec::new(),
            requirements: Vec::new(),
            catalogs: Catalogs {
                default: Catalog(BTreeMap::new()),
                named: BTreeMap::new(),
            },
            overrides: BTreeMap::new(),
        }),
    },
    Case {
        name: CaseName("truncated_json_is_refused"),
        word: Word::Refuses,
        input: || "{",
        expect: Expect::Error(()),
    },
];

/// Checks `adapter` against the port's contract: the shared rows, a claim, and facts about the package beside the manifest read.
pub(crate) fn check(adapter: &impl ManifestReadingPort) {
    let subject = |text: Input| {
        adapter
            .declared(
                &ManifestFile {
                    path: FilePath("package.json".into()),
                    text: ManifestText(text.into()),
                },
                &[],
            )
            .map_err(|error| match error {
                crate::outbound::manifest_reading::errors::ManifestReadingError::Unreadable(
                    _message,
                ) => (),
            })
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
    // An adapter claims manifest files, and its facts are about a package manifest in the directory of the one it read.
    assert!(
        !adapter.claims().is_empty(),
        "an adapter claims at least one manifest file"
    );
    let manifest = ManifestFile {
        path: FilePath("packages/core/package.json".into()),
        text: ManifestText("{}".into()),
    };
    assert_eq!(
        directory(RepositoryPath(&adapter.about(&manifest).0)),
        directory(RepositoryPath(&manifest.path.0)),
        "the facts of a manifest are about a package manifest in its own directory"
    );
}
