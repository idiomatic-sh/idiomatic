//! Package manifest shape validation, dependency precedence and workspace forms.

use super::schemas::ManifestSchema;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::packages::types::Declared;
use ryo::languages::{DependencyKind, FilePath, ManifestFile, ManifestText, Optionality};

type Input = (&'static str, &'static str);
/// Members, requirements, and where each requirement says its package comes from.
type Value = (usize, usize, &'static str);
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("workspace_array_selects_packages"),
        word: Word::Succeeds,
        input: || ("package.json", r#"{"workspaces":["packages/*"]}"#),
        expect: Expect::Value((1, 0, "")),
    },
    Case {
        name: CaseName("optional_dependency_overrides_normal"),
        word: Word::Succeeds,
        input: || {
            (
                "package.json",
                r#"{"dependencies":{"core":"*"},"optionalDependencies":{"core":"*"}}"#,
            )
        },
        expect: Expect::Value((0, 1, "Registry")),
    },
    Case {
        name: CaseName("each_dependency_says_its_registry_its_path_its_git_repository_or_its_url"),
        word: Word::Succeeds,
        input: || {
            (
                "packages/app/package.json",
                r#"{"dependencies":{"a":"^1","b":"file:../b","c":"github:owner/c","d":"workspace:*","e":"https://example.com/e.tgz","f":"owner/f#main","g":"npm:other@2","h":"../h"}}"#,
            )
        },
        expect: Expect::Value((0, 8, "Registry Path Git Workspace Url Git Registry Path")),
    },
    Case {
        name: CaseName("package_json_rejects_comments"),
        word: Word::Refuses,
        input: || ("package.json", "{ // comment\n}"),
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("dependency_versions_must_be_strings"),
        word: Word::Refuses,
        input: || ("package.json", r#"{"dependencies":{"x":42}}"#),
        expect: Expect::Error(()),
    },
];
fn subject((path, text): Input) -> Result<Value, Failure> {
    let declared = Declared::try_from(ManifestSchema {
        manifest: &ManifestFile {
            path: FilePath(path.into()),
            text: ManifestText(text.into()),
        },
        candidates: &[FilePath("packages/core/package.json".into())],
    })
    .map_err(|error| match error {
        crate::outbound::manifest_reading::errors::ManifestReadingError::Unreadable(_message) => (),
    })?;
    if path == "package.json" && !declared.requirements.is_empty() {
        assert_eq!(declared.requirements[0].kind, DependencyKind::Normal);
        assert_eq!(declared.requirements[0].optional, Optionality::Optional);
    }
    let origins = declared
        .requirements
        .iter()
        .map(|requirement| format!("{:?}", requirement.origin))
        .collect::<Vec<_>>()
        .join(" ");
    Ok((
        declared.members.len(),
        declared.requirements.len(),
        &*Box::leak(origins.into_boxed_str()),
    ))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
