//! The `type_facts` service's cases: the basic type of each expression statement of a source.

use super::service::resolutions;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::node::service::{id_of, typed_nodes};
use crate::fixtures::syntax_files::with_file;
use ruff_python_ast::AnyNodeRef;

/// A source.
type Input = &'static str;
/// The type of each expression statement of the source, in the order they are written, joined by `|`.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_literal_and_a_display_have_the_type_they_build"),
        word: Word::Succeeds,
        input: || "1\n'a'\n[a]\n{a: b}\nf'{a}'\n(a for a in b)\n",
        expect: Expect::Value(
            "Atom(Integer)|Atom(String)|Atom(List)|Atom(Dict)|Atom(String)|Atom(Generator)",
        ),
    },
    Case {
        name: CaseName("an_operation_follows_the_numeric_tower_and_refuses_what_python_refuses"),
        word: Word::Succeeds,
        input: || "1 + 1.0\nTrue + 1\n1 / 2\n'a' + 1\n-'a'\nnot a\n'a' % b\n",
        expect: Expect::Value(
            "Atom(Float)|Atom(Integer)|Atom(Float)|TypeError|TypeError|Atom(Boolean)|Atom(String)",
        ),
    },
    Case {
        name: CaseName("branches_are_a_union_and_an_included_type_folds_into_the_wider"),
        word: Word::Succeeds,
        input: || "1 if a else 'b'\n1 if a else 2.0\n1 or 'a' or None\na or 1\n",
        expect: Expect::Value(
            "Union([Integer, String])|Atom(Float)|Union([Integer, String, Null])|Unknown",
        ),
    },
];

fn subject(source: Input) -> Result<Value, Failure> {
    let mut shown: Vec<String> = Vec::new();
    with_file(source, &mut |file| {
        let mut types = Vec::new();
        resolutions(file, &mut types);
        for numbered in typed_nodes(file) {
            if let AnyNodeRef::StmtExpr(statement) = numbered.node
                && let Some(found) =
                    id_of(&*statement.value).and_then(|id| types.get(usize::from(id)))
            {
                shown.push(format!("{found:?}"));
            }
        }
    });
    Ok(Box::leak(shown.join("|").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
