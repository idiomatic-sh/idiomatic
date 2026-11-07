//! The `analyzed_source` item's cases: the files examined become its rows.

use ryo::languages;

use super::item::AnalyzedSourceItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::links::types::Linked;
use crate::domain::module_tree::types::{ModuleText, SourceCount, Unfollowed};
use crate::domain::package_index::types::ListIndex;
use crate::domain::paths::types::RepoPath;
use crate::fixtures::reading::SHOWN;
use ryo::languages::contract::want_file;
use ryo::languages::contract::{Reading, written};

/// What the case writes.
type Input = fn() -> Linked;
/// The rows the item wrote, every cell of each.
type Value = Reading;
type Failure = ();

fn three_files() -> Linked {
    Linked {
        dependencies: Vec::new(),
        analyzed: vec![ListIndex(1), ListIndex(4), ListIndex(9)],
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
const FILES: [&str; 10] = [
    "src/0.rs", "src/1.rs", "src/2.rs", "src/3.rs", "src/4.rs", "src/5.rs", "src/6.rs", "src/7.rs",
    "src/8.rs", "src/9.rs",
];

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_file_examined_is_a_row"),
        word: Word::Succeeds,
        input: || three_files,
        expect: Expect::Value(Reading {
            rows: &[
                &[want_file("src/1.rs")],
                &[want_file("src/4.rs")],
                &[want_file("src/9.rs")],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_module_without_a_file_is_read_in_part_at_its_file"),
        word: Word::Succeeds,
        input: || a_gap,
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &["src/5.rs declares module `tests` with no source files (in src/5.rs)"],
        }),
    },
    Case {
        name: CaseName("no_file_examined_is_no_row"),
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
    AnalyzedSourceItem::rows(&linked(), &mut symbols, &mut out);
    written(
        out,
        symbols,
        (&FILES, &[]),
        (
            languages::SourcePath(SHOWN),
            languages::Relation::AnalyzedSource,
        ),
        &[],
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
