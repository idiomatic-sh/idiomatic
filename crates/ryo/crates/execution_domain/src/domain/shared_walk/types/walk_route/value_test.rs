//! A selection's barrier follows the kind of its input.

use super::value::{WalkBarrier, source_barrier};
use crate::domain::executable_program::types::InputSource;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::{
    plan_parts::types::{Input, StepId},
    plans::types::Operation,
};

fn select(from: Input) -> Operation {
    Operation::Select {
        from,
        comparisons: vec![],
        kept: vec![],
    }
}

#[test]
fn a_selection_barrier_follows_the_kind_of_its_input() {
    let relation = select(Input::Relation(Relation::SyntaxTree));
    assert!(matches!(
        source_barrier(&relation, &[]),
        WalkBarrier::Representation
    ));
    let step = select(Input::Step(StepId(0)));
    assert!(matches!(
        source_barrier(&step, &[]),
        WalkBarrier::DependentInput
    ));
    assert!(matches!(
        source_barrier(
            &step,
            &[InputSource::Observation(
                crate::domain::executable_program::types::ObservationPlace(0)
            )]
        ),
        WalkBarrier::Observation
    ));
}
