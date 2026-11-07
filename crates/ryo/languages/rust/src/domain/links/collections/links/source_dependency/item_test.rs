//! The `source_dependency` item's cases: the links between the files become its rows.

use ryo::languages::{self, Offset};

use super::item::SourceDependencyItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::links::types::{SourceDependencyLink, Linked};
use crate::domain::module_tree::types::{ModuleText, SourceCount, Spelling, Unfollowed};
use crate::domain::package_index::types::ListIndex;
use crate::domain::paths::types::RepoPath;
use crate::fixtures::reading::SHOWN;
use ryo::languages::contract::{Reading, written};
use ryo::languages::contract::{want_file, want_name, want_span};

/// What the case writes.
type Input = fn() -> Linked;
/// The rows the item wrote, every cell of each.
type Value = Reading;
type Failure = ();

fn two_links() -> Linked {
    Linked {
        dependencies: vec![
            SourceDependencyLink {
                source: ListIndex(2),
                target: ListIndex(5),
                start: Offset(7),
                end: Offset(9),
                written: Spelling("b".into()),
            },
            SourceDependencyLink {
                source: ListIndex(2),
                target: ListIndex(6),
                start: Offset(11),
                end: Offset(13),
                written: Spelling("c".into()),
            },
        ],
        analyzed: vec![ListIndex(2)],
        gaps: Vec::new(),
        gap_files: Vec::new(),
        packages: None,
    }
}

fn a_gap() -> Linked {
    Linked {
        gaps: vec![Unfollowed::Files {
            file: RepoPath("src/5.rs".into()),
            module: ModuleText("tests".into()),
            count: SourceCount::None,
        }],
        gap_files: vec![languages::FileId(5)],
        ..Linked::default()
    }
}

fn nothing() -> Linked {
    Linked::default()
}

/// The path of each file by its number.
const FILES: [&str; 7] = [
    "src/0.rs", "src/1.rs", "src/2.rs", "src/3.rs", "src/4.rs", "src/5.rs", "src/6.rs",
];

/// The source the links are written in: twenty bytes on one line, so that an offset is a column.
const TEXT: &[u8] = b"use a; use b; use c;";

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_link_is_a_row_of_the_two_files_and_the_place"),
        word: Word::Succeeds,
        input: || two_links,
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_file("src/2.rs"),
                    want_file("src/5.rs"),
                    want_span((1, 8), (1, 10)),
                    want_name("b"),
                ],
                &[
                    want_file("src/2.rs"),
                    want_file("src/6.rs"),
                    want_span((1, 12), (1, 14)),
                    want_name("c"),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName(
            "a_module_without_a_file_is_read_in_part_at_its_file_and_a_file_no_crate_root_reaches_is_no_doubt",
        ),
        word: Word::Succeeds,
        input: || a_gap,
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &["src/5.rs declares module `tests` with no source files (in src/5.rs)"],
        }),
    },
    Case {
        name: CaseName("no_link_is_no_row"),
        word: Word::Succeeds,
        input: || nothing,
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];

fn subject(linked: Input) -> Value {
    let mut symbols = languages::Symbols::default();
    let mut out = languages::contract::Collect::default();
    SourceDependencyItem::rows(&linked(), &mut symbols, &mut out);
    written(
        out,
        symbols,
        (&FILES, TEXT),
        (
            languages::SourcePath(SHOWN),
            languages::Relation::SourceDependency,
        ),
        &[],
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
