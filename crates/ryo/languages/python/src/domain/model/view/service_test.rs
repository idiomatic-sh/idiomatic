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

/// No part built.
const PENDING: Built = Built {
    lexical: PartState::Pending,
    resolution: PartState::Pending,
    flow: PartState::Pending,
    reads_by_name: PartState::Pending,
    shadows_by_binding: PartState::Pending,
};

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("asking_for_nothing_builds_nothing"),
        word: Word::Succeeds,
        input: || &[ModelPart::Nothing],
        expect: Expect::Value((PENDING, 0)),
    },
    Case {
        name: CaseName("the_lexical_part_is_built_without_the_flow"),
        word: Word::Succeeds,
        input: || &[ModelPart::Lexical],
        expect: Expect::Value((Built { lexical: PartState::Ready, ..PENDING }, 0)),
    },
    Case {
        name: CaseName("the_resolution_is_built_only_when_asked_for"),
        word: Word::Succeeds,
        input: || &[ModelPart::Resolution],
        expect: Expect::Value((Built { lexical: PartState::Ready, resolution: PartState::Ready, ..PENDING }, 0)),
    },
    Case {
        name: CaseName("the_flow_part_builds_the_lexical_part_first"),
        word: Word::Succeeds,
        input: || &[ModelPart::Flow, ModelPart::Lexical],
        expect: Expect::Value((Built { lexical: PartState::Ready, flow: PartState::Ready, ..PENDING }, 4)),
    },
    Case {
        name: CaseName("the_reads_are_listed_by_name_only_when_one_name_is_asked_for"),
        word: Word::Succeeds,
        input: || &[ModelPart::ReadsByName],
        expect: Expect::Value((
            Built { lexical: PartState::Ready, flow: PartState::Ready, reads_by_name: PartState::Ready, ..PENDING },
            4,
        )),
    },
    Case {
        name: CaseName("the_shadows_are_listed_by_binding_only_when_one_binding_is_asked_for"),
        word: Word::Succeeds,
        input: || &[ModelPart::Flow, ModelPart::ShadowsByBinding],
        expect: Expect::Value((
            Built { lexical: PartState::Ready, flow: PartState::Ready, shadows_by_binding: PartState::Ready, ..PENDING },
            4,
        )),
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
