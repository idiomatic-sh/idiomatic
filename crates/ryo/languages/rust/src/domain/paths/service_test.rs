//! The `paths` service's cases, on values alone.

use super::service::{
    directory_of, entries_among, join, lexical_join, manifest_in, within, written,
};
use super::types::{Lying, Relative};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::paths::types::{EntryKind, RepoPath};

/// What a row of the cases does with paths.
#[derive(Clone, Copy)]
enum Scenario {
    /// Places a relative path under a directory.
    Join(&'static str, &'static str),
    /// Places a relative path as it is written.
    Written(&'static str, &'static str),
    /// The directory of a path.
    Directory(&'static str),
    /// The manifest in a directory.
    Manifest(&'static str),
    /// Whether a path lies below a directory.
    Within(&'static str, &'static str),
    /// A relative path under a directory, resolved by its dots alone.
    Lexical(&'static str, &'static str),
    /// The directory of a file, written as text.
    FileDirectory(&'static str),
    /// What a directory holds among the files a request lists.
    Given(&'static str),
}

/// What a row of the cases does.
type Input = Scenario;
/// The path it makes, or yes and no.
type Value = &'static str;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_directory_given_holds_each_name_below_it_once"),
        word: Word::Succeeds,
        input: || Scenario::Given("b"),
        expect: Expect::Value("Cargo.toml src/"),
    },
    Case {
        name: CaseName("the_root_given_holds_its_files_and_directories"),
        word: Word::Succeeds,
        input: || Scenario::Given(""),
        expect: Expect::Value("Cargo.toml b/ bb/"),
    },
    Case {
        name: CaseName("dots_are_resolved_without_regard_to_the_repository"),
        word: Word::Succeeds,
        input: || Scenario::Lexical("a/b", "../c/./d.rs"),
        expect: Expect::Value("a/c/d.rs"),
    },
    Case {
        name: CaseName("a_file_at_the_top_has_the_root_as_its_directory"),
        word: Word::Succeeds,
        input: || Scenario::FileDirectory("c.rs"),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_file_below_has_its_parent_as_its_directory"),
        word: Word::Succeeds,
        input: || Scenario::FileDirectory("a/b/c.rs"),
        expect: Expect::Value("a/b"),
    },
    Case {
        name: CaseName("a_path_climbing_above_the_directory_is_resolved"),
        word: Word::Succeeds,
        input: || Scenario::Join("crates/app", "../lib/./x"),
        expect: Expect::Value("crates/lib/x"),
    },
    Case {
        name: CaseName("a_path_climbing_out_of_the_repository_is_nothing"),
        word: Word::Succeeds,
        input: || Scenario::Join("app", "../../x"),
        expect: Expect::Value("outside"),
    },
    Case {
        name: CaseName("a_declared_path_keeps_its_dots_but_not_its_leading_one"),
        word: Word::Succeeds,
        input: || Scenario::Written("cli", "./../dist/x.rs"),
        expect: Expect::Value("cli/../dist/x.rs"),
    },
    Case {
        name: CaseName("the_directory_of_a_path_of_the_root_is_the_root"),
        word: Word::Succeeds,
        input: || Scenario::Directory("Cargo.toml"),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("the_manifest_of_the_root_is_a_path_of_the_root"),
        word: Word::Succeeds,
        input: || Scenario::Manifest(""),
        expect: Expect::Value("Cargo.toml"),
    },
    Case {
        name: CaseName("a_sibling_with_the_same_prefix_is_not_below"),
        word: Word::Succeeds,
        input: || Scenario::Within("crates/app2/x", "crates/app"),
        expect: Expect::Value("no"),
    },
];

fn subject(scenario: Input) -> Value {
    let path = |text: &str| RepoPath(text.into());
    let shown = |made: RepoPath| -> &'static str { Box::leak(made.0) };
    match scenario {
        Scenario::Given(directory) => {
            let files: Vec<ryo::languages::FilePath> =
                ["b/src/lib.rs", "b/src/inner.rs", "bb/x.rs"]
                    .map(|each| ryo::languages::FilePath(each.into()))
                    .to_vec();
            let supplied =
                ["Cargo.toml", "b/Cargo.toml"].map(|each| ryo::languages::ManifestFile {
                    path: ryo::languages::FilePath(each.into()),
                    text: ryo::languages::ManifestText("".into()),
                });
            let mut names: Vec<String> = entries_among(&files, &supplied, &path(directory))
                .into_iter()
                .map(|entry| match entry.kind {
                    EntryKind::Directory => format!("{}/", entry.name.0),
                    EntryKind::File => entry.name.0.to_string(),
                })
                .collect();
            names.sort();
            Box::leak(names.join(" ").into_boxed_str())
        }
        Scenario::Join(directory, relative) => {
            join(&path(directory), &Relative(relative.into())).map_or("outside", shown)
        }
        Scenario::Written(directory, relative) => {
            shown(written(&path(directory), &Relative(relative.into())))
        }
        Scenario::Directory(file) | Scenario::FileDirectory(file) => {
            shown(directory_of(&path(file)))
        }
        Scenario::Manifest(directory) => shown(manifest_in(&path(directory))),
        Scenario::Lexical(directory, relative) => {
            shown(lexical_join(&path(directory), &Relative(relative.into())))
        }
        Scenario::Within(inside, directory) => {
            if within(&path(inside), &path(directory)) == Lying::Within {
                "yes"
            } else {
                "no"
            }
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
