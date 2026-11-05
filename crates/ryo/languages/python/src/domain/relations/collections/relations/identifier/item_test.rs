//! Canonical names and original Unicode source positions are distinct contracts.
use super::super::fixtures::contract::check;
use super::item::IdentifierItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::collections::relations::syntax_error::item::SyntaxErrorItem;
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("attribute identifiers keep owners and original UTF-8 spans"),
        word: Word::Succeeds,
        input: || "ｏｓ.ｅｎｖｉｒｏｎ\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(3),
                    want_name("os"),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 7)),
                ],
                &[
                    want_node(6),
                    want_name("environ"),
                    want_file(SHOWN),
                    want_span((1, 8), (1, 29)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("name expression retains source extent after NFKC"),
        word: Word::Succeeds,
        input: || "𝗲𝘅𝗲𝗰\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2),
                want_name("exec"),
                want_file(SHOWN),
                want_span((1, 1), (1, 17)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("ASCII identifier has same semantic shape"),
        word: Word::Succeeds,
        input: || "exec\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2),
                want_name("exec"),
                want_file(SHOWN),
                want_span((1, 1), (1, 5)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("incomplete source is not complete identifier knowledge"),
        word: Word::Succeeds,
        input: || "if:\n pass\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &["identifiers require a complete syntax tree"],
        }),
    },
];
fn subject(source: Input) -> Value {
    check::<IdentifierItem>();
    reading::<IdentifierItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}

#[test]
fn complete_ast_semantic_error_does_not_hide_identifiers() {
    let result = subject("global x\nnonlocal x\nx\n");
    assert!(result.causes.is_empty());
    assert_eq!(result.rows.len(), 3);
}

#[test]
fn incomplete_parse_still_refuses_facts_beside_preserved_validation_errors() {
    let source = "global x\nnonlocal x\nif:\n";
    let identifiers = subject(source);
    let diagnostics = reading::<SyntaxErrorItem>(source);
    assert_eq!(
        identifiers.causes,
        &["identifiers require a complete syntax tree"]
    );
    assert!(identifiers.rows.is_empty());
    assert!(diagnostics.rows.len() >= 2);
}
