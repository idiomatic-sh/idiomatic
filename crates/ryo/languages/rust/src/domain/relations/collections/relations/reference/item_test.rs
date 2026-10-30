// ignore: idiomatic.arch/file-length - one row for each shape of reference the item reads, each with its full rows
//! The `reference` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::ReferenceItem;
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
        name: CaseName("unresolved_reference_keeps_its_name_and_scope_id"),
        word: Word::Succeeds,
        input: || "fn run() { missing(); }",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(17),
                want_node(1),
                want_name("missing"),
                want_file(SHOWN),
                want_span((1, 12), (1, 19)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("every_outermost_path_is_a_reference"),
        word: Word::Succeeds,
        input: || "fn run(value: crate::Value) { consume(value); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(15),
                    want_node(1),
                    want_name("crate::Value"),
                    want_file(SHOWN),
                    want_span((1, 15), (1, 27)),
                ],
                &[
                    want_node(33),
                    want_node(1),
                    want_name("consume"),
                    want_file(SHOWN),
                    want_span((1, 31), (1, 38)),
                ],
                &[
                    want_node(40),
                    want_node(1),
                    want_name("value"),
                    want_file(SHOWN),
                    want_span((1, 39), (1, 44)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_path_of_a_use_is_an_import_not_a_reference"),
        word: Word::Succeeds,
        input: || "use crate::service::Handle;\nstatic HANDLE: Handle = Handle::new();",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(29),
                    want_node(0),
                    want_name("Handle"),
                    want_file(SHOWN),
                    want_span((2, 16), (2, 22)),
                ],
                &[
                    want_node(38),
                    want_node(0),
                    want_name("Handle::new"),
                    want_file(SHOWN),
                    want_span((2, 25), (2, 36)),
                ],
            ],
            causes: &[],
        }),
    },
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
    Case {
        name: CaseName("the_arguments_of_assert_are_expressions_whose_paths_are_references"),
        word: Word::Succeeds,
        input: || "fn f() { assert!(a, \"{}\", b); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(17),
                    want_node(1),
                    want_name("assert"),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 16)),
                ],
                &[
                    want_node(36),
                    want_node(1),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((1, 18), (1, 19)),
                ],
                &[
                    want_node(43),
                    want_node(1),
                    want_name("b"),
                    want_file(SHOWN),
                    want_span((1, 27), (1, 28)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_arguments_of_assert_eq_are_expressions_whose_paths_are_references"),
        word: Word::Succeeds,
        input: || "fn f() { assert_eq!(a, b); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(17),
                    want_node(1),
                    want_name("assert_eq"),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 19)),
                ],
                &[
                    want_node(33),
                    want_node(1),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((1, 21), (1, 22)),
                ],
                &[
                    want_node(38),
                    want_node(1),
                    want_name("b"),
                    want_file(SHOWN),
                    want_span((1, 24), (1, 25)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_arguments_of_assert_ne_are_expressions_whose_paths_are_references"),
        word: Word::Succeeds,
        input: || "fn f() { assert_ne!(a, b); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(17),
                    want_node(1),
                    want_name("assert_ne"),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 19)),
                ],
                &[
                    want_node(33),
                    want_node(1),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((1, 21), (1, 22)),
                ],
                &[
                    want_node(38),
                    want_node(1),
                    want_name("b"),
                    want_file(SHOWN),
                    want_span((1, 24), (1, 25)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_arguments_of_debug_assert_are_expressions_whose_paths_are_references"),
        word: Word::Succeeds,
        input: || "fn f() { debug_assert!(a); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(17),
                    want_node(1),
                    want_name("debug_assert"),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 22)),
                ],
                &[
                    want_node(30),
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
        name: CaseName(
            "the_arguments_of_debug_assert_eq_are_expressions_whose_paths_are_references",
        ),
        word: Word::Succeeds,
        input: || "fn f() { debug_assert_eq!(a, b); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(17),
                    want_node(1),
                    want_name("debug_assert_eq"),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 25)),
                ],
                &[
                    want_node(33),
                    want_node(1),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((1, 27), (1, 28)),
                ],
                &[
                    want_node(38),
                    want_node(1),
                    want_name("b"),
                    want_file(SHOWN),
                    want_span((1, 30), (1, 31)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName(
            "the_arguments_of_debug_assert_ne_are_expressions_whose_paths_are_references",
        ),
        word: Word::Succeeds,
        input: || "fn f() { debug_assert_ne!(a, b); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(17),
                    want_node(1),
                    want_name("debug_assert_ne"),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 25)),
                ],
                &[
                    want_node(33),
                    want_node(1),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((1, 27), (1, 28)),
                ],
                &[
                    want_node(38),
                    want_node(1),
                    want_name("b"),
                    want_file(SHOWN),
                    want_span((1, 30), (1, 31)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_arguments_of_vec_are_expressions_whose_paths_are_references"),
        word: Word::Succeeds,
        input: || "fn f() { vec![a; n]; }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(17),
                    want_node(1),
                    want_name("vec"),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 13)),
                ],
                &[
                    want_node(33),
                    want_node(1),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((1, 15), (1, 16)),
                ],
                &[
                    want_node(38),
                    want_node(1),
                    want_name("n"),
                    want_file(SHOWN),
                    want_span((1, 18), (1, 19)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_arguments_of_matches_are_expressions_whose_paths_are_references"),
        word: Word::Succeeds,
        input: || "fn f() { matches!(a, Some(b)); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(17),
                    want_node(1),
                    want_name("matches"),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 17)),
                ],
                &[
                    want_node(37),
                    want_node(1),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((1, 19), (1, 20)),
                ],
                &[
                    want_node(42),
                    want_node(1),
                    want_name("Some"),
                    want_file(SHOWN),
                    want_span((1, 22), (1, 26)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("the_arguments_of_panic_are_expressions_whose_paths_are_references"),
        word: Word::Succeeds,
        input: || "fn f() { panic!(\"{}\", a); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(17),
                    want_node(1),
                    want_name("panic"),
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
        name: CaseName("the_arguments_of_todo_are_expressions_whose_paths_are_references"),
        word: Word::Succeeds,
        input: || "fn f() { todo!(\"{}\", a); }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(17),
                    want_node(1),
                    want_name("todo"),
                    want_file(SHOWN),
                    want_span((1, 10), (1, 14)),
                ],
                &[
                    want_node(35),
                    want_node(1),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((1, 22), (1, 23)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName(
            "the_tokens_of_a_macro_the_language_does_not_read_as_expressions_are_no_references",
        ),
        word: Word::Succeeds,
        input: || "fn f() { unknown!(a, b); }",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(17),
                want_node(1),
                want_name("unknown"),
                want_file(SHOWN),
                want_span((1, 10), (1, 17)),
            ]],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<ReferenceItem>();
    reading::<ReferenceItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
