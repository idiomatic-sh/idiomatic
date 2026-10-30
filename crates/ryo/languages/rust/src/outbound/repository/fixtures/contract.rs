//! What every `repository` adapter must pass.

use super::super::interface::RepositoryPort;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::paths::service::present;
use crate::domain::paths::types::Entry;
use crate::domain::paths::types::EntryKind;
use crate::domain::paths::types::EntryName;
use crate::domain::paths::types::Height;
use crate::domain::paths::types::Look;
use crate::domain::paths::types::Placement;
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::Section;
use crate::domain::targets::types::WrittenPath;

/// The files the contract's repository holds: the path and the text of each.
pub(crate) const FILES: &[(&str, &str)] = &[
    ("marker", "top"),
    ("app/marker", "app"),
    ("app/Cargo.toml", "[package]\nname = \"app\"\n"),
    ("app/src/lib.rs", ""),
];

/// What a row of the contract asks about the repository.
#[derive(Clone, Copy)]
pub(crate) enum Scenario {
    /// The text of a file.
    Text(&'static str),
    /// What is in a directory.
    Entries(&'static str),
    /// The files called `marker` from a directory up.
    Above(&'static str),
    /// Where a path lands that a file of the directory some height above another writes.
    Placed(&'static str, u32, &'static str),
}

/// What a row of the contract is given.
type Input = Scenario;
/// What a row of the contract expects, shown.
type Value = &'static str;
/// A repository port refuses nothing: what it cannot read is absent.
type Failure = ();

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub(crate) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_file_is_read"),
        word: Word::Succeeds,
        input: || Scenario::Text("app/Cargo.toml"),
        expect: Expect::Value("[package]\nname = \"app\"\n"),
    },
    Case {
        name: CaseName("a_file_that_is_not_there_has_no_text"),
        word: Word::Succeeds,
        input: || Scenario::Text("app/missing"),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_directory_lists_its_files_and_directories"),
        word: Word::Succeeds,
        input: || Scenario::Entries("app"),
        expect: Expect::Value("Cargo.toml marker src/"),
    },
    Case {
        name: CaseName("a_path_written_in_a_directory_lands_below_it"),
        word: Word::Succeeds,
        input: || Scenario::Placed("app", 0, "out"),
        expect: Expect::Value("app/out"),
    },
    Case {
        name: CaseName("a_path_written_in_a_directory_above_lands_below_that_one"),
        word: Word::Succeeds,
        input: || Scenario::Placed("app", 1, "target"),
        expect: Expect::Value("target"),
    },
    Case {
        name: CaseName("a_path_written_above_the_root_lands_outside_the_repository"),
        word: Word::Succeeds,
        input: || Scenario::Placed("app", 2, "target"),
        expect: Expect::Value("outside"),
    },
    Case {
        name: CaseName("a_file_is_looked_for_in_the_directory_and_each_one_above"),
        word: Word::Succeeds,
        input: || Scenario::Above("app"),
        expect: Expect::Value("app top"),
    },
];

/// What the adapter answers for the scenario, shown.
fn shown(adapter: &impl RepositoryPort, scenario: Scenario) -> &'static str {
    let text = match scenario {
        Scenario::Placed(directory, height, written) => match adapter.placed(
            &RepoPath(directory.into()),
            Height(height),
            &WrittenPath(written.into()),
        ) {
            Placement::Inside(path) => path.to_string(),
            Placement::Outside => "outside".to_owned(),
        },
        Scenario::Text(path) => present(adapter.text(&RepoPath(path.into())))
            .map_or_else(String::new, |text| text.to_string()),
        Scenario::Entries(directory) => {
            let mut names: Vec<String> = adapter
                .entries(&RepoPath(directory.into()))
                .0
                .into_iter()
                .map(|Entry { name, kind }| {
                    if kind == EntryKind::Directory {
                        format!("{name}/")
                    } else {
                        name.to_string()
                    }
                })
                .collect();
            names.sort();
            names.join(" ")
        }
        Scenario::Above(directory) => adapter
            .above(&RepoPath(directory.into()), &EntryName("marker".into()))
            .0
            .into_iter()
            .take(2)
            .filter_map(|Look { text, .. }| match text {
                Section::Present(text) => Some(text.to_string()),
                Section::Absent => None,
            })
            .collect::<Vec<_>>()
            .join(" "),
    };
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Box::leak(text.into_boxed_str())
}

/// Runs the contract's rows against one implementation holding [`FILES`] below the root it was built for: the fake, or an adapter.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub(crate) fn check(adapter: &impl RepositoryPort) {
    run(CASES, Word::Succeeds, |scenario| {
        Ok(shown(adapter, scenario))
    });
}
