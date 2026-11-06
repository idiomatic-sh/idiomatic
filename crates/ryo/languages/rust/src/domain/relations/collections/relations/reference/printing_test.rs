//! The `reference` item's cases for printing.

use crate::fixtures::reading::SHOWN;
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};
use crate::cases::{Case, CaseName, Expect, Word, run};
use super::item_test::subject;

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and the causes for which it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_arguments_of_format_are_expressions_whose_paths_are_references"),
        word: Word::Succeeds,
        input: || "fn f() { format!(\"{}\", a); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(17),
                    want_node(1),
                    want_name("format"),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 16)),
                ],
                &[
                    want_node(35),
                    want_node(1),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((1, 24), (1, 25)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_arguments_of_print_are_expressions_whose_paths_are_references"),
        word: Word::Succeeds,
        input: || "fn f() { print!(\"{}\", a); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(17),
                    want_node(1),
                    want_name("print"),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 15)),
                ],
                &[
                    want_node(35),
                    want_node(1),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((1, 23), (1, 24)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_arguments_of_println_are_expressions_whose_paths_are_references"),
        word: Word::Succeeds,
        input: || "fn f() { println!(\"{}\", a); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(17),
                    want_node(1),
                    want_name("println"),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 17)),
                ],
                &[
                    want_node(35),
                    want_node(1),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((1, 25), (1, 26)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_arguments_of_eprint_are_expressions_whose_paths_are_references"),
        word: Word::Succeeds,
        input: || "fn f() { eprint!(\"{}\", a); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(17),
                    want_node(1),
                    want_name("eprint"),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 16)),
                ],
                &[
                    want_node(35),
                    want_node(1),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((1, 24), (1, 25)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_arguments_of_eprintln_are_expressions_whose_paths_are_references"),
        word: Word::Succeeds,
        input: || "fn f() { eprintln!(\"{}\", a); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(17),
                    want_node(1),
                    want_name("eprintln"),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 18)),
                ],
                &[
                    want_node(35),
                    want_node(1),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((1, 26), (1, 27)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_arguments_of_write_are_expressions_whose_paths_are_references"),
        word: Word::Succeeds,
        input: || "fn f() { write!(w, \"{}\", a); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(17),
                    want_node(1),
                    want_name("write"),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 15)),
                ],
                &[
                    want_node(36),
                    want_node(1),
                    want_name("w"),
                    want_file(SHOWN),
                    want_span((1, 17), (1, 18)),
                ],
                &[
                    want_node(43),
                    want_node(1),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((1, 26), (1, 27)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_arguments_of_writeln_are_expressions_whose_paths_are_references"),
        word: Word::Succeeds,
        input: || "fn f() { writeln!(w, \"{}\", a); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(17),
                    want_node(1),
                    want_name("writeln"),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 17)),
                ],
                &[
                    want_node(36),
                    want_node(1),
                    want_name("w"),
                    want_file(SHOWN),
                    want_span((1, 19), (1, 20)),
                ],
                &[
                    want_node(43),
                    want_node(1),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((1, 28), (1, 29)),
                ],
            ],
            causes: &[],
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
