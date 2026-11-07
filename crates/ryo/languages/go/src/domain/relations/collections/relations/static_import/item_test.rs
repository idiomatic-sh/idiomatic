//! The `static_import` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::StaticImportItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span, want_unknown};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_import_is_a_row_by_its_path_and_the_name_it_binds"),
        word: Word::Succeeds,
        input: || "import (\n\t\"fmt\"\n\tstr \"strings\"\n\t_ \"embed\"\n\t. \"math\"\n)\nimport \"os\"\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(6),
                    want_node(0),
                    want_unknown("declared-by-package"),
                    want_name("fmt"),
                    want_file(SHOWN),
                    want_span((2, 2), (2, 7)),
                ],
                &[
                    want_node(10),
                    want_node(0),
                    want_name("str"),
                    want_name("strings"),
                    want_file(SHOWN),
                    want_span((3, 2), (3, 15)),
                ],
                &[
                    want_node(17),
                    want_node(0),
                    want_name("_"),
                    want_name("embed"),
                    want_file(SHOWN),
                    want_span((4, 2), (4, 11)),
                ],
                &[
                    want_node(24),
                    want_node(0),
                    want_unknown("dot-import"),
                    want_name("math"),
                    want_file(SHOWN),
                    want_span((5, 2), (5, 10)),
                ],
                &[
                    want_node(36),
                    want_node(0),
                    want_unknown("declared-by-package"),
                    want_name("os"),
                    want_file(SHOWN),
                    want_span((7, 8), (7, 12)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_declaration_that_is_no_import_has_no_row"),
        word: Word::Succeeds,
        input: || "var os = 1\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_import_binds_the_name_it_gives_or_the_one_its_package_declares"),
        word: Word::Succeeds,
        input: || "package p\nimport (\n\t\"a/b\"\n\tc \"d\"\n\t. \"e\"\n\t_ \"f\"\n\t\"g/v2\"\n)\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(11),
                    want_node(0),
                    want_unknown("declared-by-package"),
                    want_name("a/b"),
                    want_file(SHOWN),
                    want_span((3, 2), (3, 7)),
                ],
                &[
                    want_node(15),
                    want_node(0),
                    want_name("c"),
                    want_name("d"),
                    want_file(SHOWN),
                    want_span((4, 2), (4, 7)),
                ],
                &[
                    want_node(22),
                    want_node(0),
                    want_unknown("dot-import"),
                    want_name("e"),
                    want_file(SHOWN),
                    want_span((5, 2), (5, 7)),
                ],
                &[
                    want_node(29),
                    want_node(0),
                    want_name("_"),
                    want_name("f"),
                    want_file(SHOWN),
                    want_span((6, 2), (6, 7)),
                ],
                &[
                    want_node(36),
                    want_node(0),
                    want_unknown("declared-by-package"),
                    want_name("g/v2"),
                    want_file(SHOWN),
                    want_span((7, 2), (7, 8)),
                ],
            ],
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
