//! The `flow` service's cases: the state of each read of a parsed source, in evaluation order.

use super::service::build;
use super::types::FlowModel;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::reaching::types::{FlowReadState, FlowSource};
use crate::domain::model::view::types::{ModelPart, ModelView as _};
use crate::outbound::syntax::fixtures::model_view::{ModelFixture, viewed};
use ryo::languages::FileId;

/// The source.
type Input = &'static str;
/// What can reach each read, in evaluation order.
type Value = &'static [FlowReadState];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("both_arms_of_a_condition_reach_the_read_after_it"),
        word: Word::Succeeds,
        input: || "if c:\n    x = 1\nelse:\n    x = 2\nx\n",
        expect: Expect::Value(&[FlowReadState::Unbound, FlowReadState::Alternatives]),
    },
    Case {
        name: CaseName("a_name_bound_in_one_arm_only_is_possibly_unbound"),
        word: Word::Succeeds,
        input: || "if c:\n    x = 1\nx\n",
        expect: Expect::Value(&[FlowReadState::Unbound, FlowReadState::PossiblyUnbound]),
    },
    Case {
        name: CaseName("a_read_in_a_loop_before_a_later_binding_of_the_loop_is_uncertain"),
        word: Word::Succeeds,
        input: || "x = 0\nwhile c:\n    x\n    x = 1\n",
        expect: Expect::Value(&[FlowReadState::Unbound, FlowReadState::Uncertain]),
    },
    Case {
        name: CaseName("a_deleted_name_is_unbound"),
        word: Word::Succeeds,
        input: || "x = 1\ndel x\nx\n",
        expect: Expect::Value(&[FlowReadState::Unique, FlowReadState::Unbound]),
    },
];

fn subject(source: Input) -> Value {
    let mut flow = FlowModel::default();
    viewed(&mut ModelFixture::default(), source, FileId(0), &mut |view| {
        let tree = view.tree;
        let model = view.semantics(ModelPart::Flow).model;
        build(
            &mut flow,
            FlowSource {
                names: &model.names,
                lexical: &model.lexical,
                definitions: &model.definitions,
                schedule: &model.schedule,
                tree,
            },
        );
    });
    Box::leak(flow.reads.iter().map(|read| read.state).collect())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
