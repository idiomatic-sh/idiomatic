//! What the `proofs` service names and summarizes: the text a version another uses is named by in a digest, and a
//! version's stamp. What a scenario expected against what it found is the verdict queries' (`scenario_judging`).

use super::service::{summary, use_text, version_paths};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::files::types::{FileSize, ModifiedTime, Stamp};
use crate::domain::packages::types::{
    DefinitionKind, DefinitionName, Ownership, PackageName, Release, VersionText,
};
use crate::domain::paths::types::PathText;

/// What a row of the cases does.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// A version of a package named by its name: `kind/package/name@version`.
    Named,
    /// A version of a package only a scenario names, whose name holds where it is.
    NamedWhere,
    /// A version's paths: its files and every directory of it, its own first.
    Paths,
    /// A version's stamp: the files' sizes and the latest time a file or directory was modified.
    Stamped,
}

type Input = Scenario;
/// What the case gave, written out; empty when it is the row's.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_version_another_uses_is_named_kind_package_name_version"),
        word: Word::Succeeds,
        input: || Scenario::Named,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_package_only_a_scenario_names_is_named_with_where_it_is"),
        word: Word::Succeeds,
        input: || Scenario::NamedWhere,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_stamp_is_taken_over_the_files_and_every_directory_its_own_first"),
        word: Word::Succeeds,
        input: || Scenario::Paths,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_stamp_moves_with_a_size_or_the_latest_modified_time"),
        word: Word::Succeeds,
        input: || Scenario::Stamped,
        expect: Expect::Value(String::new()),
    },
];

/// The text `package`'s rule `r` at 0.1.0 is named by.
fn named(package: &str) -> String {
    use_text(&Release {
        package: PackageName(package.into()),
        kind: DefinitionKind::Rule,
        name: DefinitionName("r".into()),
        version: VersionText("0.1.0".into()),
        ownership: Ownership::Shared,
    })
    .0
    .to_string()
}

/// A stamp of this size and time.
fn stamp(size: u64, modified: u64) -> Stamp {
    Stamp {
        size: FileSize(size),
        modified: ModifiedTime(modified),
    }
}

fn subject(scenario: Scenario) -> Value {
    let (said, wanted) = match scenario {
        Scenario::Named => (named("p"), "rule/p/r@0.1.0".to_owned()),
        Scenario::NamedWhere => (
            named("q@/elsewhere/q"),
            "rule/q@/elsewhere/q/r@0.1.0".to_owned(),
        ),
        Scenario::Paths => {
            let paths = version_paths(
                [
                    "rules/r/0.1.0/rule.toml",
                    "rules/r/0.1.0/cases/a/b.txt",
                    "rules/r/0.2.0/rule.toml",
                    "package.toml",
                ]
                .map(PathText),
                PathText("rules/r/0.1.0"),
            );
            (
                format!(
                    "{:?} {:?}",
                    paths.files.iter().map(|file| file.0).collect::<Vec<_>>(),
                    paths
                        .directories
                        .iter()
                        .map(|directory| directory.0)
                        .collect::<Vec<_>>()
                ),
                format!(
                    "{:?} {:?}",
                    ["rules/r/0.1.0/rule.toml", "rules/r/0.1.0/cases/a/b.txt"],
                    [
                        "rules/r/0.1.0",
                        "rules/r/0.1.0/cases",
                        "rules/r/0.1.0/cases/a"
                    ]
                ),
            )
        }
        Scenario::Stamped => (
            format!(
                "{:?}",
                summary(&[stamp(3, 5), stamp(4, 7)], &[stamp(96, 9)])
            ),
            format!("{:?}", stamp(7, 9)),
        ),
    };
    if said == wanted { String::new() } else { said }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
