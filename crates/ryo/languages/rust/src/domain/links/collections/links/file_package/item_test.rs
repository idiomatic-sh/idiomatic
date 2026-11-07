//! The `file_package` item's cases: each file a package holds becomes a row with the package's manifest.

use ryo::languages;

use super::item::FilePackageItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::links::types::{Packaged, Placed};
use crate::domain::package_index::types::ListIndex;
use crate::domain::packages::types::Refusal;
use crate::domain::paths::types::RepoPath;
use crate::domain::workspaces::types::Reason;
use crate::fixtures::reading::SHOWN;
use ryo::languages::contract::want_file;
use ryo::languages::contract::{Reading, written};

/// What the case writes.
type Input = fn() -> Packaged;
/// The rows the item wrote, every cell of each, and the causes for which the relation is read in part.
type Value = Reading;
type Failure = ();

fn two_files() -> Packaged {
    Packaged {
        files: vec![
            Placed {
                place: ListIndex(3),
                package: RepoPath("app/Cargo.toml".into()),
            },
            Placed {
                place: ListIndex(6),
                package: RepoPath("Cargo.toml".into()),
            },
        ],
        refused: Vec::new(),
        listed: listing(&MANIFESTS),
    }
}

fn a_refused_manifest() -> Packaged {
    Packaged {
        files: Vec::new(),
        refused: vec![Refusal {
            manifest: RepoPath("bad/Cargo.toml".into()),
            reason: Reason("no package".into()),
        }],
        listed: std::collections::BTreeMap::new(),
    }
}

fn an_unlisted_manifest() -> Packaged {
    Packaged {
        files: vec![Placed {
            place: ListIndex(3),
            package: RepoPath("app/Cargo.toml".into()),
        }],
        refused: Vec::new(),
        // The one number after the last file the run lists.
        listed: std::collections::BTreeMap::from([(
            RepoPath("app/Cargo.toml".into()),
            languages::FileId(7),
        )]),
    }
}

/// The path of each file by its number: the manifests are files of the run too.
const FILES: [&str; 8] = [
    "Cargo.toml",
    "app/Cargo.toml",
    "src/0.rs",
    "src/1.rs",
    "src/2.rs",
    "src/3.rs",
    "src/4.rs",
    // A manifest the run does not list is numbered after the files it lists.
    "app/Cargo.toml",
];

/// The manifests the packages are named by.
const MANIFESTS: [&str; 2] = ["app/Cargo.toml", "Cargo.toml"];

/// The number of each manifest among `FILES`.
fn listing(listed: &[&str]) -> std::collections::BTreeMap<RepoPath, languages::FileId> {
    FILES
        .iter()
        .enumerate()
        .filter(|(_, path)| listed.contains(path))
        .map(|(at, path)| {
            (
                RepoPath((*path).into()),
                languages::FileId(u32::try_from(at).unwrap_or(u32::MAX)),
            )
        })
        .collect()
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_file_a_package_holds_is_a_row_of_the_file_and_the_manifest"),
        word: Word::Succeeds,
        input: || two_files,
        expect: Expect::Value(Reading {
            rows: &[
                &[want_file("src/1.rs"), want_file("app/Cargo.toml")],
                &[want_file("src/4.rs"), want_file("Cargo.toml")],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName(
            "a_manifest_the_run_does_not_list_is_a_file_numbered_after_the_ones_it_lists",
        ),
        word: Word::Succeeds,
        input: || an_unlisted_manifest,
        expect: Expect::Value(Reading {
            rows: &[&[want_file("src/1.rs"), want_file("app/Cargo.toml")]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_refused_manifest_leaves_the_relation_read_in_part"),
        word: Word::Succeeds,
        input: || a_refused_manifest,
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &["`bad/Cargo.toml` was refused: no package"],
        }),
    },
];

fn subject(packaged: Input) -> Value {
    let mut symbols = languages::Symbols::default();
    let mut out = languages::contract::Collect::default();
    FilePackageItem::rows(&packaged(), &mut symbols, &mut out);
    written(
        out,
        symbols,
        (&FILES, &[]),
        (
            languages::SourcePath(SHOWN),
            languages::Relation::FilePackage,
        ),
        &[],
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
