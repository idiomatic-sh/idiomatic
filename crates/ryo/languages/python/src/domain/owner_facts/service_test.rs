//! The `owner_facts` service's cases: the scope each name of a source is evaluated in, by Python's rules.

use super::service::{owner_of, owners};
use super::types::Owners;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::node::service::{typed_nodes, written};
use crate::domain::node::types::Numbered;
use crate::fixtures::syntax_files::with_file;
use ruff_python_ast::AnyNodeRef;
use ryo::languages::Position;

/// A source.
type Input = &'static str;
/// Each name expression of the source with the first line of its owner, in order: `name@owner`, joined by a space.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_default_and_a_decorator_are_evaluated_around_their_function"),
        word: Word::Succeeds,
        input: || "@decorator\ndef function(parameter=default):\n    return body\n",
        expect: Expect::Value("decorator@module default@module body@inner"),
    },
    Case {
        name: CaseName("only_the_first_iterable_of_a_comprehension_is_evaluated_around_it"),
        word: Word::Succeeds,
        input: || "[each for row in rows for each in row]\n",
        expect: Expect::Value("each@inner row@inner rows@module each@inner row@inner"),
    },
];

fn subject(source: Input) -> Result<Value, Failure> {
    let mut shown = Vec::new();
    with_file(source, &mut |file| {
        let mut kept = Owners::default();
        let owned = owners(file, &mut kept).to_vec();
        for Numbered { id, node } in typed_nodes(file) {
            if matches!(node, AnyNodeRef::ExprName(_)) {
                let owner = owner_of(&owned, id);
                let around = if owner == Position(0) {
                    "module"
                } else {
                    "inner"
                };
                shown.push(format!("{}@{around}", written(file, id).0));
            }
        }
    });
    Ok(Box::leak(shown.join(" ").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
