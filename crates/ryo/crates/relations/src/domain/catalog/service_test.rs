//! The `catalog` service's cases: the one list names every relation of the catalog once.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::service::all;

/// How many relations the one list of the catalog names, counted by the macro that every crate of the engine calls.
macro_rules! listed_relations {
    (
        syntax { $( $syntax:ident $syntax_module:ident $syntax_row:ident $syntax_column:ident; )* }
        manifest { $( $manifest:ident $manifest_module:ident $manifest_row:ident $manifest_column:ident; )* }
        engine { $( $engine:ident $engine_module:ident $engine_row:ident $engine_column:ident; )* }
    ) => {
        fn the_list_names_as_many_relations_as_the_catalog_has() -> usize {
            usize::from(
                [$( stringify!($syntax), )* $( stringify!($manifest), )* $( stringify!($engine), )*].len()
                    == all().len(),
            )
        }
    };
}

crate::each_relation!(listed_relations);

/// A scenario: what a row asks of the catalog's list.
type Input = fn() -> usize;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_list_names_as_many_relations_as_the_catalog_has"),
    word: Word::Succeeds,
    input: || the_list_names_as_many_relations_as_the_catalog_has,
    expect: Expect::Value(1),
}];

/// Runs one scenario; the author states it.
fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
