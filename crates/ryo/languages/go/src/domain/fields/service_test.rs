//! The `fields` service's cases: the names a field declares, its type and its tag.

use super::service::{names, type_of};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::GoNode;
use crate::domain::syntax::types::Kind;
use crate::fixtures::reading::{asked, of_kind, spans};

/// A source whose first field is asked about.
type Input = &'static str;
/// The names it declares, then its type, as text.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_field_declares_its_names_and_has_the_type_after_them"),
        word: Word::Succeeds,
        input: || "package p\ntype T struct {\n\tx, y int\n}\n",
        expect: Expect::Value("x y: int"),
    },
    Case {
        name: CaseName("a_field_with_a_tag_has_the_type_before_it"),
        word: Word::Succeeds,
        input: || "package p\ntype T struct {\n\tx int `json:\"x\"`\n}\n",
        expect: Expect::Value("x: int"),
    },
    Case {
        name: CaseName("an_embedded_field_declares_no_name_and_is_its_type"),
        word: Word::Succeeds,
        input: || "package p\ntype T struct {\n\t*Base\n}\n",
        expect: Expect::Value(": *Base"),
    },
    Case {
        name: CaseName("a_parameter_declares_its_name_and_has_its_type"),
        word: Word::Succeeds,
        input: || "package p\nfunc f(a, b int) {}\n",
        expect: Expect::Value("a b: int"),
    },
];

fn subject(source: Input) -> Value {
    let answer = asked(source, |tree| {
        of_kind(tree, Kind::Node(GoNode::Field)).first().map(|field| {
            let declared: Vec<String> = names(tree, *field).map(|each| spans(tree, each)).collect();
            let typed = type_of(tree, *field)
                .map(|each| spans(tree, each))
                .unwrap_or_default();
            format!("{}: {typed}", declared.join(" "))
        })
    });
    Box::leak(answer.unwrap_or_default().into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
