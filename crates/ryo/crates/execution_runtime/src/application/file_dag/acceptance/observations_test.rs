//! Request-qualified observation slots and the complete synthetic instruction census.
#[cfg(test)]
pub(super) mod fixtures;
use fixtures::{DeclaredFileProducers, Fixture, FrontierSlots, Names};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::{
    coverage::types::RuleIndex,
    executable_program::{service::compile, types::{InputSource, ObservationPlace}},
    file_program::types::ResolvedObservationProducer,
    operating::errors::OperatingError,
};
use ryo_values::domain::observations::types::{ObservationBindingName, ObservationColumnName};
use ryo_values::domain::records::types::ObservationIndex;

fn fixture(frontier: FrontierSlots) -> Fixture {
    fixtures::fixture(
        frontier,
        Names {
            left: ObservationBindingName("left".into()),
            right: ObservationBindingName("right".into()),
            column: ObservationColumnName("file".into()),
        },
    )
}

#[derive(Clone, Copy)]
enum Scenario {
    QualifiedSlots,
    UndeclaredFrontier,
    MissingSlot,
}
type Input = Scenario;
type Value = ();
#[derive(Debug, PartialEq)]
enum Refusal {
    ObservationIncomplete,
}
type Failure = Refusal;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("two_local_zero_inputs_keep_their_distinct_request_producers"),
        word: Word::Succeeds,
        input: || Scenario::QualifiedSlots,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("undeclared_frontier_ordinal_is_a_typed_refusal"),
        word: Word::Refuses,
        input: || Scenario::UndeclaredFrontier,
        expect: Expect::Error(Refusal::ObservationIncomplete),
    },
    Case {
        name: CaseName("declared_local_input_without_its_qualified_slot_is_a_typed_refusal"),
        word: Word::Refuses,
        input: || Scenario::MissingSlot,
        expect: Expect::Error(Refusal::ObservationIncomplete),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    let outcome = match scenario {
        Scenario::QualifiedSlots => two_local_zero_inputs_keep_their_distinct_request_producers(),
        Scenario::UndeclaredFrontier => undeclared_frontier_ordinal_is_a_typed_refusal(),
        Scenario::MissingSlot => {
            declared_local_input_without_its_qualified_slot_is_a_typed_refusal()
        }
    };
    match outcome {
        Ok(()) => Ok(()),
        Err(OperatingError::ObservationIncomplete { .. }) => Err(Refusal::ObservationIncomplete),
        Err(error) => panic!("unexpected observation refusal: {error:?}"),
    }
}

fn two_local_zero_inputs_keep_their_distinct_request_producers() -> Result<(), OperatingError> {
    let Fixture {
        plans,
        graph,
        operators,
    } = fixture(FrontierSlots::Declared);
    let metadata = graph.observations.as_ref().unwrap();
    assert_eq!(metadata.slots.len(), 2);
    assert_eq!(metadata.slots[0].rule, RuleIndex(2));
    assert_eq!(metadata.slots[1].rule, RuleIndex(3));
    assert_eq!(
        metadata.bindings[2][0].input.producers[0],
        ResolvedObservationProducer::Output {
            rule: RuleIndex(0),
            output: ObservationIndex(0)
        }
    );
    assert_eq!(
        metadata.bindings[3][0].input.producers[0],
        ResolvedObservationProducer::Output {
            rule: RuleIndex(1),
            output: ObservationIndex(0)
        }
    );
    let program = compile(&plans, graph, &DeclaredFileProducers, &operators, ryo_execution_domain::domain::matching::types::PatternMatcher(&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl)).unwrap();
    assert_eq!(
        program.rules()[2].frontier[0],
        Ok(InputSource::Observation(ObservationPlace(0)))
    );
    assert_eq!(
        program.rules()[3].frontier[0],
        Ok(InputSource::Observation(ObservationPlace(1)))
    );
    Ok(())
}

fn undeclared_frontier_ordinal_is_a_typed_refusal() -> Result<(), OperatingError> {
    let Fixture {
        plans,
        graph,
        operators,
    } = fixture(FrontierSlots::UndeclaredFirst);
    let program = compile(&plans, graph, &DeclaredFileProducers, &operators, ryo_execution_domain::domain::matching::types::PatternMatcher(&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl)).unwrap();
    assert!(matches!(
        &program.rules()[2].frontier[0],
        Err(OperatingError::ObservationIncomplete { .. })
    ));
    assert_eq!(
        program.rules()[3].frontier[0],
        Ok(InputSource::Observation(ObservationPlace(1)))
    );
    program.rules()[2].frontier[0].clone().map(|_| ())
}

fn declared_local_input_without_its_qualified_slot_is_a_typed_refusal() -> Result<(), OperatingError>
{
    let Fixture {
        plans,
        mut graph,
        operators,
    } = fixture(FrontierSlots::Declared);
    graph.observations.as_mut().unwrap().slots[0].rule = RuleIndex(99);
    let program = compile(&plans, graph, &DeclaredFileProducers, &operators, ryo_execution_domain::domain::matching::types::PatternMatcher(&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl)).unwrap();
    assert!(matches!(
        &program.rules()[2].frontier[0],
        Err(OperatingError::ObservationIncomplete { .. })
    ));
    assert_eq!(
        program.rules()[3].frontier[0],
        Ok(InputSource::Observation(ObservationPlace(1)))
    );
    program.rules()[2].frontier[0].clone().map(|_| ())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
