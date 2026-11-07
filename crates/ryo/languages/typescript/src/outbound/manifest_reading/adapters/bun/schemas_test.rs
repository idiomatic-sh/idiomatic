//! Bun catalog shapes: top level, beside the workspace packages, and neither.

use super::schemas::read;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::catalogs::service::requirement;
use crate::domain::packages::types::{Binding, Directory, Spec};
use ryo::languages::{DependencyKind, FilePath, ManifestFile, ManifestText, Optionality};

type Input = &'static str;
/// Default catalog entries, named catalogs, and where each default entry says its package comes from.
type Value = (usize, usize, &'static str);
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_catalog_entry_says_its_registry_its_path_or_its_git_repository"),
        word: Word::Succeeds,
        input: || r#"{"catalog":{"a":"^1","b":"file:../b","c":"github:owner/c"}}"#,
        expect: Expect::Value((3, 0, "Registry Path Git")),
    },
    Case {
        name: CaseName("catalogs_beside_the_workspace_packages_are_read"),
        word: Word::Succeeds,
        input: || r#"{"workspaces":{"packages":["packages/*"],"catalog":{"react":"^19"},"catalogs":{"testing":{"jest":"30"}}}}"#,
        expect: Expect::Value((1, 1, "Registry")),
    },
    Case {
        name: CaseName("top_level_catalogs_are_read"),
        word: Word::Succeeds,
        input: || r#"{"workspaces":["packages/*"],"catalog":{"react":"^19"},"catalogs":{"a":{"x":"1"},"b":{"y":"2"}}}"#,
        expect: Expect::Value((1, 2, "Registry")),
    },
    Case {
        name: CaseName("a_manifest_without_catalogs_declares_none"),
        word: Word::Succeeds,
        input: || r#"{"name":"app","dependencies":{"react":"^19"}}"#,
        expect: Expect::Value((0, 0, "")),
    },
    Case {
        name: CaseName("a_catalog_version_must_be_a_string"),
        word: Word::Refuses,
        input: || r#"{"catalog":{"react":19}}"#,
        expect: Expect::Error(()),
    },
];
fn subject(text: Input) -> Result<Value, Failure> {
    read(&ManifestFile {
        path: FilePath("package.json".into()),
        text: ManifestText(text.into()),
    })
    .map(|declared| {
        (
            declared.catalogs.default.0.len(),
            declared.catalogs.named.len(),
            origins(&declared),
        )
    })
    .map_err(|error| match error {
        crate::outbound::manifest_reading::errors::ManifestReadingError::Unreadable(_message) => (),
    })
}
/// Where each entry of the default catalog says its package comes from, once a `catalog:` requirement takes its version.
fn origins(declared: &crate::domain::packages::types::Declared) -> &'static str {
    let origins = declared
        .catalogs
        .default
        .0
        .iter()
        .map(|(name, version)| {
            format!(
                "{:?}",
                requirement(
                    Directory(""),
                    Binding(&name.0),
                    Spec(&version.0),
                    DependencyKind::Normal,
                    Optionality::Required,
                )
                .origin
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    Box::leak(origins.into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
