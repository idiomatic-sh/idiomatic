//! Cases for TypeScript comment rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::CommentItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_flag, want_name, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("comments_exclude_delimiters"),
        word: Word::Succeeds,
        input: || "// hello\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_name(" hello"),
                want_file("item.ts"),
                want_span((1, 1), (1, 9)),
                want_flag(false),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_block_opened_with_two_stars_is_documentation"),
        word: Word::Succeeds,
        input: || "/** Adds. */\n/**/\n/* plain */\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(1),
                    want_name("* Adds. "),
                    want_file("item.ts"),
                    want_span((1, 1), (1, 13)),
                    want_flag(true),
                ],
                &[
                    want_node(3),
                    want_name(""),
                    want_file("item.ts"),
                    want_span((2, 1), (2, 5)),
                    want_flag(false),
                ],
                &[
                    want_node(5),
                    want_name(" plain "),
                    want_file("item.ts"),
                    want_span((3, 1), (3, 12)),
                    want_flag(false),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_empty_file_has_no_row"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];
fn subject(source: Input) -> Value {
    check::<CommentItem>();
    reading::<CommentItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
