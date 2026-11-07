//! The `static_import` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::StaticImportItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and the causes for which it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_leaf_of_a_group_is_an_import"),
        word: Word::Succeeds,
        input: || "use std::{io::{self, Write}, fmt::*};",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(20),
                    want_node(0),
                    want_name("io"),
                    want_name("std::io"),
                    want_file(SHOWN),
                    want_span((1, 16), (1, 20)),
                ],
                &[
                    want_node(27),
                    want_node(0),
                    want_name("Write"),
                    want_name("std::io::Write"),
                    want_file(SHOWN),
                    want_span((1, 22), (1, 27)),
                ],
                &[
                    want_node(35),
                    want_node(0),
                    want_name("*"),
                    want_name("std::fmt::*"),
                    want_file(SHOWN),
                    want_span((1, 30), (1, 36)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_rename_binds_its_own_name"),
        word: Word::Succeeds,
        input: || "use crate::service as svc;",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(4),
                want_node(0),
                want_name("svc"),
                want_name("crate::service"),
                want_file(SHOWN),
                want_span((1, 5), (1, 26)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_path_that_starts_with_two_colons_keeps_them"),
        word: Word::Succeeds,
        input: || "use ::std::fmt;",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(4),
                want_node(0),
                want_name("fmt"),
                want_name("::std::fmt"),
                want_file(SHOWN),
                want_span((1, 5), (1, 15)),
            ]],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<StaticImportItem>();
    reading::<StaticImportItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
