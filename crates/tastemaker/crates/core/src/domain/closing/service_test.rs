//! The `closing` service's cases: the findings the strays the closed query decided come to.

use std::collections::BTreeSet;

use super::service::{findings, prefix_of};
use super::types::{
    ClosedDirectory, ClosedOwner, Closings, EnclosedFile, GroupNumber, Stray, StrayNumber,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::paths::types::RepoPath;
use crate::domain::scopes::types::{Identity, ScopeIndex};

/// Which directory is asked about, and which files are under a `rules-exclude`.
#[derive(Clone, Copy)]
enum Scenario {
    /// A stray in a closed directory of the repository.
    Stray,
    /// A stray at the root.
    Root,
    /// A stray under a `rules-exclude`.
    Excluded,
    /// The prefix of a directory.
    Prefix,
}

type Input = Scenario;
/// What the service answered, in words.
type Value = &'static str;
type Failure = ();

fn closings(directory: &str) -> Closings {
    Closings {
        directories: vec![ClosedDirectory {
            group: GroupNumber(0),
            directory: RepoPath(directory.into()),
            owners: vec![ClosedOwner {
                scope: ScopeIndex(1),
                identity: Identity("p/part".into()),
            }],
        }],
        candidates: vec![EnclosedFile {
            stray: StrayNumber(0),
            group: GroupNumber(0),
            path: RepoPath(if directory.is_empty() {
                "stray.txt".into()
            } else {
                format!("{directory}/stray.txt").into()
            }),
            home: prefix_of(&RepoPath(directory.into())),
        }],
        ..Closings::default()
    }
}

fn subject(scenario: Input) -> Value {
    Box::leak(said_of(scenario).into_boxed_str())
}

fn said_of(scenario: Input) -> String {
    let stray = [Stray {
        stray: StrayNumber(0),
        owner: ScopeIndex(1),
    }];
    match scenario {
        Scenario::Stray => findings(&closings("src"), &BTreeSet::new(), &stray)
            .iter()
            .map(|found| format!("{} {}", found.path.0, found.message.0))
            .collect::<Vec<_>>()
            .join("|"),
        Scenario::Root => findings(&closings(""), &BTreeSet::new(), &stray)
            .iter()
            .map(|found| found.message.0.to_string())
            .collect::<Vec<_>>()
            .join("|"),
        Scenario::Excluded => {
            let excluded = BTreeSet::from([RepoPath("src/stray.txt".into())]);
            findings(&closings("src"), &excluded, &stray)
                .iter()
                .map(|found| format!("{:?}", found.help))
                .collect::<Vec<_>>()
                .join("|")
        }
        Scenario::Prefix => format!(
            "{:?} {:?}",
            prefix_of(&RepoPath("a/b".into())),
            prefix_of(&RepoPath(Box::default()))
        ),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_stray_is_an_error_named_after_its_scope_and_directory"),
        word: Word::Succeeds,
        input: || Scenario::Stray,
        expect: Expect::Value(
            "src/stray.txt `src/stray.txt` is not placed by any template, and `p/part` at `src` is closed",
        ),
    },
    Case {
        name: CaseName("a_stray_at_the_root_names_here"),
        word: Word::Succeeds,
        input: || Scenario::Root,
        expect: Expect::Value(
            "`stray.txt` is not placed by any template, and `p/part` at `.` is closed",
        ),
    },
    Case {
        name: CaseName("a_stray_under_a_rules_exclude_says_the_key_places_nothing"),
        word: Word::Succeeds,
        input: || Scenario::Excluded,
        expect: Expect::Value(
            "Said(MessageText(\"a rules-exclude keeps rules off a path; it does not exempt a file from being placed: place it with a template or spec entry (`tm add`), or remove it\"))",
        ),
    },
    Case {
        name: CaseName("a_directory_is_a_prefix_of_the_paths_beneath_it_and_the_root_has_none"),
        word: Word::Succeeds,
        input: || Scenario::Prefix,
        expect: Expect::Value("Prefix(\"a/b/\") Prefix(\"\")"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |scenario| Ok(subject(scenario)));
}
