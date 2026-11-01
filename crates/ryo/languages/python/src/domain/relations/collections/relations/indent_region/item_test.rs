//! Public row serialization and incomplete-state behavior, independent of the domain scanner.
use super::super::fixtures::contract::check;
use super::item::IndentRegionItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_number, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("complete tokens preserve lexical regions after parser syntax errors"),
        word: Word::Succeeds,
        input: || "if:\n    pass\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(0),
                    want_node(0),
                    want_name(""),
                    want_number(0),
                    want_number(13),
                    want_file(SHOWN),
                    want_span((1, 1), (3, 1)),
                ],
                &[
                    want_node(6),
                    want_node(0),
                    want_name("    "),
                    want_number(4),
                    want_number(13),
                    want_file(SHOWN),
                    want_span((2, 1), (3, 1)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("lexical error keeps lexical regions partial"),
        word: Word::Succeeds,
        input: || "value = 'unterminated\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &["indentation regions require a complete balanced lexical stream"],
        }),
    },
    Case {
        name: CaseName("comment_only_file_has_its_complete_root_interval"),
        word: Word::Succeeds,
        input: || "# note\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(0),
                want_node(0),
                want_name(""),
                want_number(0),
                want_number(7),
                want_file(SHOWN),
                want_span((1, 1), (2, 1)),
            ]],
            causes: &[],
        }),
    },
];
fn subject(source: Input) -> Value {
    check::<IndentRegionItem>();
    reading::<IndentRegionItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
