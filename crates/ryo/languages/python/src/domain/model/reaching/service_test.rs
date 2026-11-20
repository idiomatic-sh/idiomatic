//! The `reaching` service's cases: how many names of a parsed source some step binds or unbinds.

use super::service::list_mutations;
use super::types::{FlowSource, ReachModel};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::lexical::types::TypedTree;
use crate::domain::model::view::types::{ModelPart, ModelView as _};
use crate::outbound::syntax::fixtures::model_view::{ModelFixture, viewed};
use ryo::languages::{Count, FileId};

/// The source.
type Input = &'static str;
/// How many (namespace, name) pairs have a step that binds or unbinds them.
type Value = Count;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_name_bound_twice_and_deleted_is_one_mutated_name"),
        word: Word::Succeeds,
        input: || "x = 1\nx = 2\ndel x\nprint(x)\n",
        expect: Expect::Value(Count(1)),
    },
    Case {
        name: CaseName(
            "a_name_of_the_module_the_same_name_in_a_function_and_the_function_itself_are_three",
        ),
        word: Word::Succeeds,
        input: || "x = 1\ndef f():\n    x = 2\n",
        expect: Expect::Value(Count(3)),
    },
    Case {
        name: CaseName("a_source_that_only_reads_mutates_nothing"),
        word: Word::Succeeds,
        input: || "print(x)\n",
        expect: Expect::Value(Count(0)),
    },
];

fn subject(source: Input) -> Value {
    let mut reach = ReachModel::default();
    viewed(
        &mut ModelFixture::default(),
        source,
        FileId(0),
        &mut |view| {
            let tree = TypedTree {
                table: view.table,
                nodes: view.nodes,
            };
            let model = view.semantics(ModelPart::Flow).model;
            list_mutations(
                &mut reach,
                FlowSource {
                    names: &model.names,
                    lexical: &model.lexical,
                    definitions: &model.definitions,
                    schedule: &model.schedule,
                    tree,
                },
            );
        },
    );
    reach.mutation_count
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
