//! The support of the `matching` cases.
use super::super::fixtures::{self, DeclaredFileProducers, Fixture, FrontierSlots, Names};
#[cfg(test)]
use crate::cases::{Case, CaseName, Expect, Word};
use ryo_execution_domain::domain::{
    coverage::types::RuleIndex,
    executable_program::{
        service::compile,
        types::{InputSource, ObservationPlace},
    },
    observation_schedule::types::ResolvedObservationProducer,
    operating::errors::OperatingError,
};
use ryo_values::domain::observations::types::{ObservationBindingName, ObservationColumnName};
use ryo_values::domain::records::types::ObservationIndex;
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
    frontier: FrontierSlots,
    moved_first_slot: Option<usize>,
}

pub(crate) type Input = Slots;

/// What the observation metadata and the compiled program state about the two local inputs.
#[derive(Debug, PartialEq)]
pub(super) struct Compiled {
    /// The rule of each request-qualified slot.
    slot_rules: Cow<'static, [usize]>,
    /// The first producer of rules 2 and 3's first binding.
    producers: [Option<ResolvedObservationProducer>; 2],
    /// The first frontier source of rules 2 and 3.
    frontiers: [InputSource; 2],
}

pub(crate) type Value = Compiled;

/// Why rule 2's first frontier is refused.
#[derive(Debug, PartialEq)]
pub(super) enum Refusal {
    ObservationIncomplete,
}

/// A refused rule 2 frontier, and the frontier source rule 3 still answers.
#[derive(Debug, PartialEq)]
pub(super) struct Refused {
    refusal: Refusal,
    sibling: Option<InputSource>,
}

pub(crate) type Failure = Refused;

const fn output(rule: usize) -> Option<ResolvedObservationProducer> {
    Some(ResolvedObservationProducer::Output {
        rule: RuleIndex(rule),
        output: ObservationIndex(0),
    })
}

pub(crate) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("two_local_zero_inputs_keep_their_distinct_request_producers"),
        word: Word::Succeeds,
        input: || Slots {
            frontier: FrontierSlots::Declared,
            moved_first_slot: None,
        },
        expect: Expect::Value(Compiled {
            slot_rules: Cow::Borrowed(&[2, 3]),
            producers: [output(0), output(1)],
            frontiers: [
                InputSource::Observation(ObservationPlace(0)),
                InputSource::Observation(ObservationPlace(1)),
            ],
        }),
    },
    Case {
        name: CaseName("undeclared_frontier_ordinal_is_a_typed_refusal"),
        word: Word::Refuses,
        input: || Slots {
            frontier: FrontierSlots::UndeclaredFirst,
            moved_first_slot: None,
        },
        expect: Expect::Error(Refused {
            refusal: Refusal::ObservationIncomplete,
            sibling: Some(InputSource::Observation(ObservationPlace(1))),
        }),
    },
    Case {
        name: CaseName("declared_local_input_without_its_qualified_slot_is_a_typed_refusal"),
        word: Word::Refuses,
        input: || Slots {
            frontier: FrontierSlots::Declared,
            moved_first_slot: Some(99),
        },
        expect: Expect::Error(Refused {
            refusal: Refusal::ObservationIncomplete,
            sibling: Some(InputSource::Observation(ObservationPlace(1))),
        }),
    },
];

/// Compiles the request with its slots and reads rules 2 and 3's first frontier sources.
pub(crate) fn subject(slots: Input) -> Result<Value, Failure> {
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
    let program = compile(
        &plans,
        graph,
        &DeclaredFileProducers,
        &operators,
        ryo_execution_domain::domain::matching::types::PatternMatcher(
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
        ),
    )
    .expect("the request compiles");
    let first = program.rules()[2].frontier[0].clone();
    let sibling = program.rules()[3].frontier[0].clone().ok();
    match first {
        Ok(first) => Ok(Compiled {
            slot_rules: slot_rules.into(),
            producers,
            frontiers: [first, sibling.expect("rule 3 answers a frontier")],
        }),
        Err(OperatingError::ObservationIncomplete { .. }) => Err(Refused {
            refusal: Refusal::ObservationIncomplete,
            sibling,
        }),
        Err(error) => panic!("unexpected observation refusal: {error:?}"),
    }
}
