//! The `defines` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::DefinesItem;
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
        name: CaseName("imports_constants_variables_types_and_fields_are_defined"),
        word: Word::Succeeds,
        input: || "package p\nimport (\n\t\"fmt\"\n\t. \"os\"\n\t_ \"net\"\n)\nconst c = 1\nvar v int\ntype T struct{ f int }\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(11),
                    want_name("import"),
                    want_name("import"),
                    want_unknown("declared-by-package"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((3, 2), (3, 7)),
                ],
                &[
                    want_node(15),
                    want_name("import"),
                    want_name("import"),
                    want_unknown("dot-import"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((4, 2), (4, 3)),
                ],
                &[
                    want_node(34),
                    want_name("constant"),
                    want_name("const"),
                    want_name("c"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((7, 7), (7, 8)),
                ],
                &[
                    want_node(46),
                    want_name("variable"),
                    want_name("var"),
                    want_name("v"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((8, 5), (8, 6)),
                ],
                &[
                    want_node(56),
                    want_name("struct"),
                    want_name("struct"),
                    want_name("T"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((9, 6), (9, 7)),
                ],
                &[
                    want_node(65),
                    want_name("property"),
                    want_name("field"),
                    want_name("f"),
                    want_node(56),
                    want_file(SHOWN),
                    want_span((9, 16), (9, 17)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_method_defines_its_receiver_parameters_results_and_local_names"),
        word: Word::Succeeds,
        input: || "package p\nfunc (r T) m(a, _ int, rest ...string) (n int) {\n\tx, _ := 1, 2\n\tfor k, v := range r {\n\t}\n\tswitch t := a.(type) {\n\t}\n}\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(6),
                    want_name("method"),
                    want_name("method"),
                    want_name("m"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((2, 12), (2, 13)),
                ],
                &[
                    want_node(12),
                    want_name("parameter"),
                    want_name("receiver"),
                    want_name("r"),
                    want_node(6),
                    want_file(SHOWN),
                    want_span((2, 7), (2, 8)),
                ],
                &[
                    want_node(24),
                    want_name("parameter"),
                    want_name("parameter"),
                    want_name("a"),
                    want_node(6),
                    want_file(SHOWN),
                    want_span((2, 14), (2, 15)),
                ],
                &[
                    want_node(36),
                    want_name("parameter"),
                    want_name("variadic"),
                    want_name("rest"),
                    want_node(6),
                    want_file(SHOWN),
                    want_span((2, 24), (2, 28)),
                ],
                &[
                    want_node(48),
                    want_name("parameter"),
                    want_name("result"),
                    want_name("n"),
                    want_node(6),
                    want_file(SHOWN),
                    want_span((2, 41), (2, 42)),
                ],
                &[
                    want_node(59),
                    want_name("variable"),
                    want_name("short-var"),
                    want_name("x"),
                    want_node(6),
                    want_file(SHOWN),
                    want_span((3, 2), (3, 3)),
                ],
                &[
                    want_node(76),
                    want_name("variable"),
                    want_name("range-var"),
                    want_name("k"),
                    want_node(6),
                    want_file(SHOWN),
                    want_span((4, 6), (4, 7)),
                ],
                &[
                    want_node(76),
                    want_name("variable"),
                    want_name("range-var"),
                    want_name("v"),
                    want_node(6),
                    want_file(SHOWN),
                    want_span((4, 9), (4, 10)),
                ],
                &[
                    want_node(101),
                    want_name("variable"),
                    want_name("type-switch-var"),
                    want_name("t"),
                    want_node(6),
                    want_file(SHOWN),
                    want_span((6, 9), (6, 10)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_label_is_defined_in_the_function_it_is_in"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\nL:\n\tfor {\n\t\tbreak L\n\t}\n}\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(6),
                    want_name("function"),
                    want_name("function"),
                    want_name("f"),
                    want_node(0),
                    want_file(SHOWN),
                    want_span((2, 6), (2, 7)),
                ],
                &[
                    want_node(19),
                    want_name("label"),
                    want_name("label"),
                    want_name("L"),
                    want_node(6),
                    want_file(SHOWN),
                    want_span((3, 1), (3, 2)),
                ],
            ],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<DefinesItem>();
    reading::<DefinesItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
