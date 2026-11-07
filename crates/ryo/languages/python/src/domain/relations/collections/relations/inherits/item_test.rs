//! The `inherits` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::InheritsItem;
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
    name: CaseName("bases_are_dotted_names_or_unknown"),
    word: Word::Succeeds,
    input: || "class C(Base, mod.Mixin, Generic[T], *bases, make(), metaclass=Meta): pass\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(8),
                want_node(1),
                want_name("Base"),
                want_file(SHOWN),
                want_span((1, 9), (1, 13)),
            ],
            &[
                want_node(12),
                want_node(1),
                want_name("mod.Mixin"),
                want_file(SHOWN),
                want_span((1, 15), (1, 24)),
            ],
            &[
                want_node(20),
                want_node(1),
                want_name("Generic"),
                want_file(SHOWN),
                want_span((1, 26), (1, 36)),
            ],
            &[
                want_node(29),
                want_node(1),
                want_unknown("star-base"),
                want_file(SHOWN),
                want_span((1, 38), (1, 44)),
            ],
            &[
                want_node(35),
                want_node(1),
                want_unknown("dynamic-base"),
                want_file(SHOWN),
                want_span((1, 46), (1, 52)),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<InheritsItem>();
    reading::<InheritsItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
