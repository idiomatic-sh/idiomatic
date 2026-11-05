//! The `slots` service's cases: what an identifier is where it stands, and whether it is a reference.

use super::service::{referencing, slot_of};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::fixtures::reading::{asked, of_kind, spans};

/// What a row asks of a source.
#[derive(Clone, Copy, Debug)]
enum Query {
    /// The slot of each identifier.
    Slots,
    /// What each identifier and each selection is as a reference.
    References,
}

type Input = (&'static str, Query);
/// Each identifier, or each identifier and selection, of the source with what it is.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_name_a_short_declaration_declares_is_declared_and_one_it_uses_is_used"),
        word: Word::Succeeds,
        input: || ("package p\nfunc f() { a := b }\n", Query::Slots),
        expect: Expect::Value("p:Elsewhere f:Declares a:Declares b:Uses"),
    },
    Case {
        name: CaseName("a_name_after_a_dot_and_a_label_are_no_use"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\nfunc f() {\nL:\n\tgoto L\n\tx.y = 1\n}\n",
                Query::Slots,
            )
        },
        expect: Expect::Value(
            "p:Elsewhere f:Declares L:Elsewhere L:Elsewhere x:Elsewhere y:Elsewhere",
        ),
    },
    Case {
        name: CaseName("the_key_of_a_composite_literal_is_ambiguous"),
        word: Word::Succeeds,
        input: || ("package p\nvar a = T{k: v}\n", Query::Slots),
        expect: Expect::Value("p:Elsewhere a:Declares T:Uses k:Ambiguous v:Uses"),
    },
    Case {
        name: CaseName("a_parameter_a_type_and_an_import_name_are_declared"),
        word: Word::Succeeds,
        input: || {
            (
                "package p\nimport s \"a\"\ntype T int\nfunc f(x int) {}\n",
                Query::Slots,
            )
        },
        expect: Expect::Value(
            "p:Elsewhere s:Declares T:Declares int:Uses f:Declares x:Declares int:Uses",
        ),
    },
    Case {
        name: CaseName("a_dotted_name_is_one_reference_and_the_names_in_it_are_none"),
        word: Word::Succeeds,
        input: || ("package p\nfunc f() { a.b.c = d }\n", Query::References),
        expect: Expect::Value(
            "p:Other f:Other a.b.c:Used a.b:Other a:Other b:Other c:Other d:Used",
        ),
    },
];

fn subject((source, query): Input) -> Value {
    let answer = asked(source, |tree| {
        let named = of_kind(tree, Kind::Node(Node::Ident));
        match query {
            Query::Slots => named
                .into_iter()
                .map(|each| format!("{}:{:?}", spans(tree, each), slot_of(tree, each)))
                .collect::<Vec<_>>()
                .join(" "),
            Query::References => {
                let mut everything = named;
                everything.extend(of_kind(tree, Kind::Node(Node::SelectorExpr)));
                everything.sort();
                everything
                    .into_iter()
                    .map(|each| format!("{}:{:?}", spans(tree, each), referencing(tree, each)))
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        }
    });
    Box::leak(answer.into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
