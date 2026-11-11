//! Request-qualified observation slots and the complete synthetic instruction census.
use super::super::fixtures::FrontierSlots;
use super::fixtures::{Compiled, Refusal, Refused, Slots, exercise};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::types::{InputSource, ObservationPlace};
use ryo_execution_domain::domain::observation_schedule::types::ResolvedObservationProducer;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::records::types::ObservationIndex;
use std::borrow::Cow;

type Input = Slots;
type Value = Compiled;
type Failure = Refused;

const fn output(rule: usize) -> Option<ResolvedObservationProducer> {
    Some(ResolvedObservationProducer::Output {
        rule: RuleIndex(rule),
        output: ObservationIndex(0),
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
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
            relations_asked: Cow::Borrowed(&[
                Relation::LanguageTarget,
                Relation::CompilerFeature,
                Relation::LanguageTarget,
                Relation::CompilerFeature,
            ]),
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
            relations_asked: Cow::Borrowed(&[
                Relation::LanguageTarget,
                Relation::CompilerFeature,
                Relation::LanguageTarget,
                Relation::CompilerFeature,
            ]),
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
            relations_asked: Cow::Borrowed(&[
                Relation::LanguageTarget,
                Relation::CompilerFeature,
                Relation::LanguageTarget,
                Relation::CompilerFeature,
            ]),
        }),
    },
];

/// Compiles the request with its slots and reads rules 2 and 3's first frontier sources.
fn subject(slots: Input) -> Result<Value, Failure> {
    exercise(slots)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
