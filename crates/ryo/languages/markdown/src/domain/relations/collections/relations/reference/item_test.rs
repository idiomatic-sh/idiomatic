//! The `reference` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::ReferenceItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("links_and_images_point_at_paths_of_the_run"),
    word: Word::Succeeds,
    input: || "# T\n\n[a](../guide.md#setup) ![i](img.png) [r] [s](https://x.org) <https://y.org>\n\n[r]: other.md\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(8),
                want_node(2),
                want_name("../guide.md#setup"),
                want_file(SHOWN),
                want_span((3, 1), (3, 23)),
            ],
            &[
                want_node(13),
                want_node(2),
                want_name("img.png"),
                want_file(SHOWN),
                want_span((3, 24), (3, 37)),
            ],
            &[
                want_node(18),
                want_node(2),
                want_name("other.md"),
                want_file(SHOWN),
                want_span((3, 38), (3, 41)),
            ],
            &[
                want_node(23),
                want_node(2),
                want_name("https://x.org"),
                want_file(SHOWN),
                want_span((3, 42), (3, 60)),
            ],
            &[
                want_node(28),
                want_node(2),
                want_name("https://y.org"),
                want_file(SHOWN),
                want_span((3, 61), (3, 76)),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<ReferenceItem>();
    reading::<ReferenceItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
