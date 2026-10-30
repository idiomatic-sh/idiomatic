//! YAML workspace shape validation, member globs and catalogs.

use super::schemas::read;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::catalogs::service::requirement;
use crate::domain::packages::types::{Binding, Directory, Spec};
use ryo::languages::{DependencyKind, FilePath, ManifestFile, ManifestText, Optionality};

type Input = &'static str;
/// Members, default catalog entries, named catalogs, and where each default entry says its package comes from.
type Value = (usize, usize, usize, &'static str);
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("packages_globs_select_members_and_negation_excludes"),
        word: Word::Succeeds,
        input: || "packages:\n  - 'packages/*'\n  - '!packages/skip'\n",
        expect: Expect::Value((1, 0, 0, "")),
    },
    Case {
        name: CaseName("default_and_named_catalogs_are_read"),
        word: Word::Succeeds,
        input: || "catalog:\n  react: ^19.0.0\ncatalogs:\n  legacy:\n    react: ^17.0.0\n    jest: 29.0.0\n",
        expect: Expect::Value((0, 1, 1, "Registry")),
    },
    Case {
        name: CaseName("an_empty_file_declares_nothing"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value((0, 0, 0, "")),
    },
    Case {
        name: CaseName("a_catalog_entry_says_its_registry_its_path_or_its_git_repository"),
        word: Word::Succeeds,
        input: || "catalog:\n  a: ^1.0.0\n  b: file:../b\n  c: git+https://example.com/c.git\n",
        expect: Expect::Value((0, 3, 0, "Registry Path Git")),
    },
    Case {
        name: CaseName("a_catalog_version_must_be_a_string"),
        word: Word::Refuses,
        input: || "catalog:\n  react:\n    - 19\n",
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("malformed_yaml_is_refused"),
        word: Word::Refuses,
        input: || "packages: [\n",
        expect: Expect::Error(()),
    },
];
fn subject(text: Input) -> Result<Value, Failure> {
    let declared = read(
        &ManifestFile {
            path: FilePath("pnpm-workspace.yaml".into()),
            text: ManifestText(text.into()),
        },
        &[
            FilePath("packages/core/package.json".into()),
            FilePath("packages/skip/package.json".into()),
            FilePath("other/package.json".into()),
            FilePath("packages/core/tsconfig.json".into()),
        ],
    )
    .map_err(|error| match error {
        crate::outbound::manifest_reading::errors::ManifestReadingError::Unreadable(_message) => (),
    })?;
    Ok((
        declared.members.len(),
        declared.catalogs.default.0.len(),
        declared.catalogs.named.len(),
        origins(&declared),
    ))
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
