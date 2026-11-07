//! The `static_import` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::StaticImportItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span, want_unknown};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("imports_bind_names_by_the_path_they_import"),
    word: Word::Succeeds,
    input: || "import a.b\nimport a.b as c\nfrom . import x\nfrom ..m import y as z\nfrom m import *\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(4),
                want_node(0),
                want_name("a"),
                want_name("a.b"),
                want_file(SHOWN),
                want_span((1, 8), (1, 11)),
            ],
            &[
                want_node(13),
                want_node(0),
                want_name("c"),
                want_name("a.b"),
                want_file(SHOWN),
                want_span((2, 8), (2, 16)),
            ],
            &[
                want_node(31),
                want_node(0),
                want_name("x"),
                want_name(".x"),
                want_file(SHOWN),
                want_span((3, 15), (3, 16)),
            ],
            &[
                want_node(45),
                want_node(0),
                want_name("z"),
                want_name("..m.y"),
                want_file(SHOWN),
                want_span((4, 17), (4, 23)),
            ],
            &[
                want_node(62),
                want_node(0),
                want_unknown("wildcard-import"),
                want_name("m"),
                want_file(SHOWN),
                want_span((5, 15), (5, 16)),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<StaticImportItem>();
    reading::<StaticImportItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
