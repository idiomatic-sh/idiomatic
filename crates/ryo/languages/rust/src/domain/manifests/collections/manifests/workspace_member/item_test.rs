//! The `workspace_member` item's cases: the packages the manifests declare become its rows, each cell read back.

use crate::domain::conditions::types::PlatformCondition;
use crate::domain::paths::types::Section;
use crate::domain::targets::types::BindingName;
use crate::domain::targets::types::PackageName;
use crate::domain::targets::types::{Checkout, Inheritance};
use ryo::languages;
use ryo::languages::Optionality;

use super::item::WorkspaceMemberItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::packages::service::numbered;
use crate::domain::packages::types::{Dependency, Located, Membership, Packages};
use crate::domain::paths::types::RepoPath;
use crate::fixtures::reading::SHOWN;
use ryo::languages::contract::want_file;
use ryo::languages::contract::{Reading, written};

/// What the case writes.
type Input = fn() -> Located;
/// The rows the item wrote, every cell of each.
type Value = Reading;
type Failure = ();

/// The case's packages, with the manifests the run lists numbered by their places among the files and the others after them.
fn located(listed: &[&str]) -> Located {
    let files: Vec<languages::FilePath> = listed
        .iter()
        .map(|path| languages::FilePath((*path).into()))
        .collect();
    let packages = packages();
    Located {
        listed: numbered(&files, &packages),
        packages,
    }
}

/// The path of each file by its number, as the rows are read back.
fn names_of(located: &Located) -> Vec<String> {
    let mut names = vec![String::new(); located.listed.len()];
    for (path, number) in &located.listed {
        let at = usize::try_from(number.0).unwrap_or(usize::MAX);
        if let Some(slot) = names.get_mut(at) {
            slot.clone_from(&path.0.to_string());
        }
    }
    names
}

fn two_packages() -> Located {
    located(&["Cargo.toml", "app/Cargo.toml", "tool/Cargo.toml"])
}

fn the_tool_unlisted() -> Located {
    located(&["Cargo.toml", "app/Cargo.toml"])
}

fn packages() -> Packages {
    Packages {
        members: vec![
            Membership {
                workspace: RepoPath("Cargo.toml".into()),
                package: RepoPath("app/Cargo.toml".into()),
            },
            Membership {
                workspace: RepoPath("Cargo.toml".into()),
                package: RepoPath("tool/Cargo.toml".into()),
            },
        ],
        dependencies: vec![
            Dependency {
                source: RepoPath("app/Cargo.toml".into()),
                target: Some(RepoPath("support/Cargo.toml".into())),
                origin: languages::DependencyOrigin::Path,
                binding: BindingName("helper".into()),
                package: PackageName("support".into()),
                kind: languages::DependencyKind::Normal,
                condition: PlatformCondition::default(),
                optional: Optionality::Optional,
                requirement: Section::Absent,
                checkout: Checkout::default(),
                inheritance: Inheritance::Own,
                replacement: Section::Absent,
            },
            Dependency {
                source: RepoPath("app/Cargo.toml".into()),
                target: Some(RepoPath("tool/Cargo.toml".into())),
                origin: languages::DependencyOrigin::Path,
                binding: BindingName("tool".into()),
                package: PackageName("tool".into()),
                kind: languages::DependencyKind::Build,
                condition: PlatformCondition("cfg(unix)".into()),
                optional: Optionality::Required,
                requirement: Section::Absent,
                checkout: Checkout::default(),
                inheritance: Inheritance::Own,
                replacement: Section::Absent,
            },
        ],
        ..Packages::default()
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_member_is_a_row_of_the_files_of_its_manifests_in_column_order"),
        word: Word::Succeeds,
        input: || two_packages,
        expect: Expect::Value(Reading {
            rows: &[
                &[want_file("Cargo.toml"), want_file("app/Cargo.toml")],
                &[want_file("Cargo.toml"), want_file("tool/Cargo.toml")],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName(
            "a_manifest_the_run_does_not_list_is_a_file_numbered_after_the_ones_it_lists",
        ),
        word: Word::Succeeds,
        input: || the_tool_unlisted,
        expect: Expect::Value(Reading {
            rows: &[
                &[want_file("Cargo.toml"), want_file("app/Cargo.toml")],
                &[want_file("Cargo.toml"), want_file("tool/Cargo.toml")],
            ],
            causes: &[],
        }),
    },
];

fn subject(given: Input) -> Value {
    let located = given();
    let names = names_of(&located);
    let shown: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut symbols = languages::Symbols::default();
    let mut out = languages::contract::Collect::default();
    WorkspaceMemberItem::rows(&located, &mut symbols, &mut out);
    written(
        out,
        symbols,
        (&shown, &[]),
        (
            languages::SourcePath(SHOWN),
            languages::Relation::WorkspaceMember,
        ),
        &[],
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
