//! The `view` service's cases: which parts of the model one ask builds.

use super::service::built;
use super::types::{Built, ModelPart, PartState, SemanticModel};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::syntax::fixtures::model_view::{ModelFixture, viewed};
use ryo::languages::FileId;

/// The parts asked for, in order.
type Input = &'static [ModelPart];
/// What is built after the asks, and how many steps the schedule has.
type Value = (Built, usize);
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("asking_for_nothing_builds_nothing"),
        word: Word::Succeeds,
        input: || &[ModelPart::Nothing],
        expect: Expect::Value((Built { lexical: PartState::Pending, resolution: PartState::Pending, flow: PartState::Pending }, 0)),
    },
    Case {
        name: CaseName("the_lexical_part_is_built_without_the_flow"),
        word: Word::Succeeds,
        input: || &[ModelPart::Lexical],
        expect: Expect::Value((Built { lexical: PartState::Ready, resolution: PartState::Pending, flow: PartState::Pending }, 0)),
    },
    Case {
        name: CaseName("the_resolution_is_built_only_when_asked_for"),
        word: Word::Succeeds,
        input: || &[ModelPart::Resolution],
        expect: Expect::Value((Built { lexical: PartState::Ready, resolution: PartState::Ready, flow: PartState::Pending }, 0)),
    },
    Case {
        name: CaseName("the_flow_part_builds_the_lexical_part_first"),
        word: Word::Succeeds,
        input: || &[ModelPart::Flow, ModelPart::Lexical],
        expect: Expect::Value((Built { lexical: PartState::Ready, resolution: PartState::Pending, flow: PartState::Ready }, 4)),
    },
];

fn subject(parts: Input) -> Value {
    let mut model = SemanticModel::default();
    let mut state = Built::default();
    viewed(&mut ModelFixture::default(), "x = y\n", FileId(0), &mut |view| {
        for part in parts {
            built(&mut model, &mut state, *part, view.tree);
        }
    });
    (state, model.schedule.steps.len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
