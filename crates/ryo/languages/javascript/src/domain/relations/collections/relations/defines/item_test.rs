//! defines reader cases and the collection contract.
use super::super::fixtures::contract::check;
use super::item::DefinesItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_declaration_binds_its_names"),
        word: Word::Succeeds,
        input: || "import d, {e as f} from \"m\";\nconst [g] = h;\nfunction k() {}\nclass C { m() {} p = 1; }\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(4),
                    want_name("import"),
                    want_name("import-default-specifier"),
                    want_name("d"),
                    want_node(0),
                    want_file("item.js"),
                    want_span((1, 8), (1, 9)),
                ],
                &[
                    want_node(10),
                    want_name("import"),
                    want_name("import-specifier"),
                    want_name("f"),
                    want_node(0),
                    want_file("item.js"),
                    want_span((1, 17), (1, 18)),
                ],
                &[
                    want_node(29),
                    want_name("constant"),
                    want_name("variable-declarator"),
                    want_name("g"),
                    want_node(0),
                    want_file("item.js"),
                    want_span((2, 8), (2, 9)),
                ],
                &[
                    want_node(42),
                    want_name("function"),
                    want_name("function-declaration"),
                    want_name("k"),
                    want_node(0),
                    want_file("item.js"),
                    want_span((3, 10), (3, 11)),
                ],
                &[
                    want_node(54),
                    want_name("class"),
                    want_name("class-declaration"),
                    want_name("C"),
                    want_node(0),
                    want_file("item.js"),
                    want_span((4, 7), (4, 8)),
                ],
                &[
                    want_node(63),
                    want_name("method"),
                    want_name("method-definition"),
                    want_name("m"),
                    want_node(54),
                    want_file("item.js"),
                    want_span((4, 11), (4, 12)),
                ],
                &[
                    want_node(74),
                    want_name("property"),
                    want_name("property-definition"),
                    want_name("p"),
                    want_node(54),
                    want_file("item.js"),
                    want_span((4, 18), (4, 19)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("empty_source_has_no_defines"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(Reading {
            rows: &[],
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
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
