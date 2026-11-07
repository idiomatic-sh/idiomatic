//! Cases for TypeScript decorates rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::DecoratesItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_number, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("class_decorator"),
        word: Word::Succeeds,
        input: || "@sealed class C {}",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2),
                want_node(1),
                want_number(0),
                want_name("sealed"),
                want_name(""),
                want_file("item.ts"),
                want_span((1, 1), (1, 8)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("arguments_of_a_call"),
        word: Word::Succeeds,
        input: || "@dec(\"arg\", 2) class C {}",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2),
                want_node(1),
                want_number(0),
                want_name("dec"),
                want_name("\"arg\", 2"),
                want_file("item.ts"),
                want_span((1, 1), (1, 15)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_member_call"),
        word: Word::Succeeds,
        input: || "@a.b() class C {}",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2),
                want_node(1),
                want_number(0),
                want_name("a.b"),
                want_name(""),
                want_file("item.ts"),
                want_span((1, 1), (1, 7)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_generic_call"),
        word: Word::Succeeds,
        input: || "@dec<T>() class C {}",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2),
                want_node(1),
                want_number(0),
                want_name("dec"),
                want_name(""),
                want_file("item.ts"),
                want_span((1, 1), (1, 10)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_comment_before_the_parentheses"),
        word: Word::Succeeds,
        input: || "@dec /* ( */ (\"arg\", 2) class C {}",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2),
                want_node(1),
                want_number(0),
                want_name("dec"),
                want_name("\"arg\", 2"),
                want_file("item.ts"),
                want_span((1, 1), (1, 24)),
            ]],
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
    check::<DecoratesItem>();
    reading::<DecoratesItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
