//! The support of the `matching` cases.
use super::super::fixtures::{self, DeclaredFileProducers, Fixture, FrontierSlots, Names};
use ryo_execution_domain::domain::{
    coverage::types::RuleIndex,
    interpreted_program::{service::compile, types::InputSource},
    observation_schedule::types::ResolvedObservationProducer,
    operating::errors::OperatingError,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::observations::types::{ObservationBindingName, ObservationColumnName};
use std::borrow::Cow;

pub(super) fn fixture(frontier: FrontierSlots) -> Fixture {
    fixtures::fixture(
        frontier,
        Names {
            left: ObservationBindingName("left".into()),
            right: ObservationBindingName("right".into()),
            column: ObservationColumnName("file".into()),
        },
    )
}

/// The request's frontier slots, and the rule the first declared slot is moved to when it is.
#[derive(Clone, Copy)]
pub(super) struct Slots {
    pub(super) frontier: FrontierSlots,
    pub(super) moved_first_slot: Option<usize>,
}

/// What the observation metadata and the compiled program state about the two local inputs.
#[derive(Debug, PartialEq)]
pub(super) struct Compiled {
    /// The rule of each request-qualified slot.
    pub(super) slot_rules: Cow<'static, [usize]>,
    /// The first producer of rules 2 and 3's first binding.
    pub(super) producers: [Option<ResolvedObservationProducer>; 2],
    /// The first frontier source of rules 2 and 3.
    pub(super) frontiers: [InputSource; 2],
    /// The relations the producers were asked to partition, in order.
    pub(super) relations_asked: Cow<'static, [Relation]>,
}

/// Why rule 2's first frontier is refused.
#[derive(Debug, PartialEq)]
pub(super) enum Refusal {
    ObservationIncomplete,
}

/// A refused rule 2 frontier, and the frontier source rule 3 still answers.
#[derive(Debug, PartialEq)]
pub(super) struct Refused {
    pub(super) refusal: Refusal,
    pub(super) sibling: Option<InputSource>,
    /// The relations the producers were asked to partition, in order.
    pub(super) relations_asked: Cow<'static, [Relation]>,
}

/// Compiles the request with its slots and reads rules 2 and 3's first frontier sources.
pub(super) fn exercise(slots: Slots) -> Result<Compiled, Refused> {
    let Fixture {
        plans,
        mut graph,
        operators,
    } = fixture(slots.frontier);
    let metadata = graph
        .observations
        .as_mut()
        .expect("the fixture declares observations");
    if let Some(rule) = slots.moved_first_slot {
        metadata.slots[0].rule = RuleIndex(rule);
    }
    let slot_rules: Vec<usize> = metadata.slots.iter().map(|slot| slot.rule.0).collect();
    let producers = [2, 3].map(|rule| {
        metadata
            .bindings
            .get(rule)
            .and_then(|bindings| bindings.first())
            .and_then(|binding| binding.input.producers.first())
            .cloned()
    });
    let partitions = DeclaredFileProducers::default();
    let program = compile(
        &plans,
        graph,
        &partitions,
        &operators,
        ryo_operators::domain::patterns::types::PatternMatcher(
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
        ),
    )
    .expect("the request compiles");
    let relations_asked = Cow::Owned(partitions.asked.borrow().clone());
    let first = program.rules()[2].frontier[0].clone();
    let sibling = program.rules()[3].frontier[0].clone().ok();
    match first {
        Ok(first) => Ok(Compiled {
            slot_rules: slot_rules.into(),
            producers,
            frontiers: [first, sibling.expect("rule 3 answers a frontier")],
            relations_asked,
        }),
        Err(OperatingError::ObservationIncomplete { .. }) => Err(Refused {
            refusal: Refusal::ObservationIncomplete,
            sibling,
            relations_asked,
        }),
        Err(error) => panic!("unexpected observation refusal: {error:?}"),
    }
}
