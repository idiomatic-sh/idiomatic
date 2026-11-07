//! What the `create` use case promises, against the fakes of the ports it calls.

use std::sync::Arc;

use super::errors::CreateError;
use super::interface::CreateUseCase as _;
use super::types::{CreateRequest, Destination, Extension, ScenarioExtension};
use super::use_case::CreateUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::files::types::{FileText, MachinePath};
use crate::domain::packages::types::{DefinitionKind, DefinitionName};
use crate::domain::paths::types::RepoPath;
use crate::domain::writing::types::Writing;
use crate::inbound::use_cases::check::fixtures::{Workspace, package, repository_of, selection};
use crate::outbound::repository::fixtures::fake::RepositoryFake;

/// The repository a row runs over.
#[derive(Clone, Copy, Debug)]
enum Repository {
    /// The workspace of the check fixtures, as `Workspace` says.
    Of(Workspace),
    /// A workspace that names no package.
    NoPackage,
    /// The check fixtures' workspace, the rule to create already written.
    Existing,
    /// The check fixtures' workspace, the rule written: its scenario files end in the extension the row names.
    Written,
}

/// The repository, the kind, the name created and the extension named for its scenario files.
type Input = (
    Repository,
    DefinitionKind,
    &'static str,
    Option<&'static str>,
);
/// The file written.
type Value = String;
type Failure = CreateError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rule_is_created_at_its_first_version"),
        word: Word::Succeeds,
        input: || {
            (
                Repository::Of(Workspace::Placed),
                DefinitionKind::Rule,
                "no-tsv",
                Some("txt"),
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_rules_scenarios_end_in_the_extension_named"),
        word: Word::Succeeds,
        input: || {
            (
                Repository::Written,
                DefinitionKind::Rule,
                "no-tsv",
                Some("py"),
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_templates_scenarios_end_in_the_extension_named"),
        word: Word::Succeeds,
        input: || {
            (
                Repository::Written,
                DefinitionKind::Template,
                "notes",
                Some("md"),
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_rule_with_no_extension_named_is_refused_never_guessed"),
        word: Word::Refuses,
        input: || {
            (
                Repository::Of(Workspace::Placed),
                DefinitionKind::Rule,
                "no-tsv",
                None,
            )
        },
        expect: Expect::Refusal(|error| {
            selection(matches!(error, CreateError::NoExtension { .. }))
        }),
    },
    Case {
        name: CaseName("a_template_with_no_extension_named_is_refused_never_guessed"),
        word: Word::Refuses,
        input: || {
            (
                Repository::Of(Workspace::Placed),
                DefinitionKind::Template,
                "notes",
                None,
            )
        },
        expect: Expect::Refusal(|error| {
            selection(matches!(error, CreateError::NoExtension { .. }))
        }),
    },
    Case {
        name: CaseName("a_template_is_created_at_its_first_version"),
        word: Word::Succeeds,
        input: || {
            (
                Repository::Of(Workspace::Placed),
                DefinitionKind::Template,
                "errors",
                Some("txt"),
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_workspace_that_names_no_package_is_refused"),
        word: Word::Refuses,
        input: || {
            (
                Repository::NoPackage,
                DefinitionKind::Rule,
                "r",
                Some("txt"),
            )
        },
        expect: Expect::error(CreateError::NoPackage),
    },
    Case {
        name: CaseName("a_definition_that_exists_is_refused"),
        word: Word::Refuses,
        input: || {
            (
                Repository::Existing,
                DefinitionKind::Rule,
                "no-tsv",
                Some("txt"),
            )
        },
        expect: Expect::Refusal(|error| selection(matches!(error, CreateError::Exists { .. }))),
    },
    Case {
        name: CaseName("a_workspace_file_the_manifest_cannot_read_fails_the_create"),
        word: Word::Fails,
        input: || {
            (
                Repository::Of(Workspace::Broken),
                DefinitionKind::Rule,
                "no-tsv",
                Some("txt"),
            )
        },
        expect: Expect::Refusal(|error| selection(matches!(error, CreateError::Manifest { .. }))),
    },
];

/// The repository fake a row runs over.
fn repository(repository: Repository) -> RepositoryFake {
    match repository {
        Repository::Of(workspace) => repository_of(workspace),
        Repository::NoPackage => RepositoryFake::of(&[("tastemaker.toml", "")]),
        Repository::Written => repository_of(Workspace::Placed),
        Repository::Existing => {
            let files = repository_of(Workspace::Placed);
            drop(
                files
                    .files
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(
                        RepoPath("rules/no-tsv/0.1.0/rule.toml".into()),
                        FileText("".into()),
                    ),
            );
            files
        }
    }
}

fn subject((given, kind, name, extension): Input) -> Result<Value, Failure> {
    let held = Arc::new(repository(given));
    let create = CreateUseCaseImpl::new(
        Arc::<RepositoryFake>::clone(&held),
        Arc::new(crate::outbound::manifest::factory::toml()),
        Arc::new(package()),
    );
    let report = create.create(CreateRequest {
        start: MachinePath("/fake".into()),
        kind,
        name: DefinitionName(name.into()),
        into: Destination::Workspace,
        extension: extension.map_or(Extension::Unnamed, |extension| {
            Extension::Named(ScenarioExtension(extension.into()))
        }),
        writing: if matches!(given, Repository::Written) {
            Writing::Write
        } else {
            Writing::DryRun
        },
    })?;
    if matches!(given, Repository::Written) {
        use crate::outbound::repository::interface::RepositoryPort as _;
        let written = held.open(&report.package).read(&report.file)?;
        let named = format!(".{}\"", extension.unwrap_or_default());
        return Ok(
            if written.0.contains(&named) && !written.0.contains("<extension>") {
                String::new()
            } else {
                written.0.into_string()
            },
        );
    }
    let expected = match kind {
        DefinitionKind::Rule => "rules/no-tsv/0.1.0/rule.toml",
        DefinitionKind::Spec => "specs/errors/0.1.0/spec.toml",
        DefinitionKind::Template => "templates/errors/0.1.0/template.toml",
    };
    Ok(
        if &*report.file.0 == expected && &*report.package.0 == "/p" {
            String::new()
        } else {
            format!("{} {}", report.package.0, report.file.0)
        },
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
