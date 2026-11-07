//! Project configuration shape validation: JSONC comments, references and unrelated options.

use super::schemas::read;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::languages::{FilePath, ManifestFile, ManifestText};

type Input = &'static str;
/// How many references the configuration declares, its base, its aliases with their paths, and the configurations it extends.
type Value = (usize, &'static str);
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("comments_and_trailing_comma_in_config"),
        word: Word::Succeeds,
        input: || "{ // comment\n\"references\": [{\"path\":\"packages/core\"}], }",
        expect: Expect::Value((1, "")),
    },
    Case {
        name: CaseName("paths_are_not_package_dependencies"),
        word: Word::Succeeds,
        input: || r#"{"compilerOptions":{"paths":{"@core/*":["src/*"]}}}"#,
        expect: Expect::Value((0, "base None; @core/* > src/*; extends ")),
    },
    Case {
        name: CaseName("paths_are_found_from_the_base_url_and_extends_follows_a_path_only"),
        word: Word::Succeeds,
        input: || r#"{"extends":["./base","@tsconfig/node20/tsconfig.json"],"compilerOptions":{"baseUrl":"./src","paths":{"@core/*":["core/*","../shared/*"]}}}"#,
        expect: Expect::Value((
            0,
            "base Some(\"src\"); @core/* > src/core/* shared/*; extends base.json",
        )),
    },
    Case {
        name: CaseName("config_rejects_unquoted_property"),
        word: Word::Refuses,
        input: || "{ references: [] }",
        expect: Expect::Error(()),
    },
];
fn subject(text: Input) -> Result<Value, Failure> {
    read(&ManifestFile {
        path: FilePath("tsconfig.json".into()),
        text: ManifestText(text.into()),
    })
    .map(|declared| {
        let aliases: Vec<String> = declared
            .aliases
            .iter()
            .map(|alias| {
                let targets: Vec<&str> = alias.targets.iter().map(|each| &*each.0).collect();
                format!("{} > {}", alias.pattern.0, targets.join(" "))
            })
            .collect();
        let extends: Vec<&str> = declared.extends.iter().map(|each| &*each.0).collect();
        let shown = if aliases.is_empty() && extends.is_empty() && declared.base_url.is_none() {
            String::new()
        } else {
            format!(
                "base {:?}; {}; extends {}",
                declared.base_url.as_ref().map(|each| &*each.0),
                aliases.join(", "),
                extends.join(" ")
            )
        };
        // The answer is leaked: a case compares it with text that lives for the whole run.
        (
            declared.references.len(),
            &*Box::leak(shown.into_boxed_str()),
        )
    })
    .map_err(|error| match error {
        crate::outbound::manifest_reading::errors::ManifestReadingError::Unreadable(_message) => (),
    })
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
