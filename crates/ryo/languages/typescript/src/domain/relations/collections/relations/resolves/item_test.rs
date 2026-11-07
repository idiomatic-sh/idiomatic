//! The `resolves` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::ResolvesItem;
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
    name: CaseName(
        "an_import_a_declaration_and_a_type_parameter_are_bound_and_a_qualified_or_unknown_name_is_not",
    ),
    word: Word::Succeeds,
    input: || "import { Record } from './types';\ninterface Local { id: number }\ninterface Stored<T> {\n  record: Record;\n  items: Array<Record>;\n  extra: T;\n  local: Local;\n  other: Missing;\n  q: ns.Record;\n}\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(60),
                want_node(39),
                want_name("Record"),
                want_node(6),
                want_file(SHOWN),
                want_span((4, 11), (4, 17)),
            ],
            &[
                want_node(76),
                want_node(39),
                want_name("Record"),
                want_node(6),
                want_file(SHOWN),
                want_span((5, 16), (5, 22)),
            ],
            &[
                want_node(88),
                want_node(39),
                want_name("T"),
                want_node(46),
                want_file(SHOWN),
                want_span((6, 10), (6, 11)),
            ],
            &[
                want_node(99),
                want_node(39),
                want_name("Local"),
                want_node(19),
                want_file(SHOWN),
                want_span((7, 10), (7, 15)),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<ResolvesItem>();
    reading::<ResolvesItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
