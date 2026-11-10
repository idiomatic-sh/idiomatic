//! What every `manifest` adapter must pass.

use crate::domain::paths::types::FileText;
use crate::domain::targets::types::Inheritance;
use ryo::languages::DependencyKind;
use ryo::languages::Optionality;

use super::super::errors::ManifestError;
use super::super::interface::ManifestPort;
use crate::cases::{Case, CaseName, Expect, Matched, Word, run, selects};
use crate::domain::paths::types::Section;
use crate::domain::targets::types::Config;
use crate::domain::targets::types::Manifest;

/// What a row of the contract is given: the text of a manifest.
type Input = &'static str;
/// What a row of the contract expects when it succeeds: the manifest, shown.
type Value = &'static str;
/// What a row of the contract expects when it is refused.
type Failure = ManifestError;

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub(crate) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_package_is_read_with_its_name"),
        word: Word::Succeeds,
        input: || "[package]\nname = \"app\"\n",
        expect: Expect::Value("package app"),
    },
    Case {
        name: CaseName("text_that_declares_nothing_is_refused"),
        word: Word::Refuses,
        input: || "",
        expect: Expect::Refusal(|error| {
            selects(Matched(
                matches!(error, ManifestError::Unreadable { reason } if !reason.is_empty()),
            ))
        }),
    },
];

/// The contract's rows about configurations.
pub(crate) const CONFIGURATION_CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_configuration_names_where_the_outputs_go"),
        word: Word::Succeeds,
        input: || "[build]\ntarget-dir = \"out\"\n",
        expect: Expect::Value("target out"),
    },
    Case {
        name: CaseName("a_configuration_that_names_no_directory_names_none"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value("target"),
    },
];

/// A part of a manifest that may be absent, as its text.
fn text<T>(section: &Section<T>) -> String
where
    T: std::ops::Deref<Target = str>,
{
    match section {
        Section::Absent => String::new(),
        Section::Present(text) => text.to_string(),
    }
}

/// The texts of a list, joined by commas.
fn texts<T>(list: &[T]) -> String
where
    T: std::ops::Deref<Target = str>,
{
    list.iter()
        .map(|each| &**each)
        .collect::<Vec<_>>()
        .join(",")
}

/// A manifest as one line of words.
pub(crate) fn shown(manifest: &Manifest) -> &'static str {
    let mut words: Vec<String> = Vec::new();
    if let Section::Present(package) = &manifest.package {
        words.push(format!("package {}", package.name));
    }
    if let Section::Present(workspace) = &manifest.workspace {
        words.push(format!(
            "workspace members {} exclude {}",
            texts(&workspace.members),
            texts(&workspace.exclude)
        ));
    }
    for each in &manifest.dependencies {
        let kind = match each.kind {
            DependencyKind::Normal => "Normal",
            DependencyKind::Dev => "Dev",
            DependencyKind::Build => "Build",
            DependencyKind::Peer => "Peer",
        };
        words.push(format!(
            "dependency {} {kind} {} {} {}{}",
            each.key,
            text(&each.platform),
            text(&each.path),
            text(&each.package),
            if each.optional == Optionality::Optional {
                " optional"
            } else {
                ""
            }
        ));
    }
    if manifest.inherits == Inheritance::Workspace {
        words.push("inherits".to_owned());
    }
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Box::leak(words.join(" ").into_boxed_str())
}

/// A configuration as one line of words.
fn configured(config: &Config) -> &'static str {
    match &config.target_dir {
        Section::Absent => "target",
        // The answer is leaked: a case compares it with text that lives for the whole run.
        Section::Present(directory) => Box::leak(format!("target {directory}").into_boxed_str()),
    }
}

/// Runs the contract's rows against one implementation: the fake, or an adapter.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub(crate) fn check(adapter: &impl ManifestPort) {
    run(CASES, Word::Succeeds, |text| {
        adapter
            .read(&FileText(text.into()))
            .map(|manifest| shown(&manifest))
    });
    run(CASES, Word::Refuses, |text| {
        adapter
            .read(&FileText(text.into()))
            .map(|manifest| shown(&manifest))
    });
    run(CONFIGURATION_CASES, Word::Succeeds, |text| {
        adapter
            .configuration(&FileText(text.into()))
            .map(|config| configured(&config))
    });
}
