//! Request-qualified observation slots and the complete synthetic instruction census.
use super::fixtures::{DeclaredFileProducers, Fixture, FrontierSlots, Names};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    coverage::types::RuleIndex,
    executable_program::{service::compile, types::InputSource},
    file_program::types::ResolvedObservationProducer,
    operating::errors::OperatingError,
};
use ryo_values::domain::observations::types::{
    ObservationBindingName, ObservationColumnName, ObservationIndex,
};

fn fixture(frontier: FrontierSlots) -> Fixture {
    super::fixtures::fixture(
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
    UnionCensus,
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
    Case {
        name: CaseName("shared_walk_census_includes_every_synthetic_observation_union"),
        word: Word::Succeeds,
        input: || Scenario::UnionCensus,
        expect: Expect::Value(()),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    let outcome = match scenario {
        Scenario::QualifiedSlots => two_local_zero_inputs_keep_their_distinct_request_producers(),
        Scenario::UndeclaredFrontier => undeclared_frontier_ordinal_is_a_typed_refusal(),
        Scenario::MissingSlot => {
            declared_local_input_without_its_qualified_slot_is_a_typed_refusal()
        }
        Scenario::UnionCensus => shared_walk_census_includes_every_synthetic_observation_union(),
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
    assert_eq!(metadata.slots[0].rule, 2);
    assert_eq!(metadata.slots[1].rule, 3);
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
    let program = compile(&plans, graph, &DeclaredFileProducers, &operators).unwrap();
    assert_eq!(
        program.rules[2].frontier[0],
        Ok(InputSource::Observation(0))
    );
    assert_eq!(
        program.rules[3].frontier[0],
        Ok(InputSource::Observation(1))
    );
    Ok(())
}

fn undeclared_frontier_ordinal_is_a_typed_refusal() -> Result<(), OperatingError> {
    let Fixture {
        plans,
        graph,
        operators,
    } = fixture(FrontierSlots::UndeclaredFirst);
    let program = compile(&plans, graph, &DeclaredFileProducers, &operators).unwrap();
    assert!(matches!(
        &program.rules[2].frontier[0],
        Err(OperatingError::ObservationIncomplete { .. })
    ));
    assert_eq!(
        program.rules[3].frontier[0],
        Ok(InputSource::Observation(1))
    );
    program.rules[2].frontier[0].clone().map(|_| ())
}

fn declared_local_input_without_its_qualified_slot_is_a_typed_refusal() -> Result<(), OperatingError>
{
    let Fixture {
        plans,
        mut graph,
        operators,
    } = fixture(FrontierSlots::Declared);
    graph.observations.as_mut().unwrap().slots[0].rule = 99;
    let program = compile(&plans, graph, &DeclaredFileProducers, &operators).unwrap();
    assert!(matches!(
        &program.rules[2].frontier[0],
        Err(OperatingError::ObservationIncomplete { .. })
    ));
    assert_eq!(
        program.rules[3].frontier[0],
        Ok(InputSource::Observation(1))
    );
    program.rules[2].frontier[0].clone().map(|_| ())
}

fn shared_walk_census_includes_every_synthetic_observation_union() -> Result<(), OperatingError> {
    let Fixture {
        plans,
        graph,
        operators,
    } = fixture(FrontierSlots::Declared);
    let program = compile(&plans, graph, &DeclaredFileProducers, &operators).unwrap();
    let instructions = program
        .regions()
        .iter()
        .flat_map(|region| region.instructions.iter());
    let total = instructions.clone().count();
    let unions = instructions
        .filter(|instruction| instruction.logical.0 >= program.logical_regions().len())
        .count();
    assert_eq!(unions, 2, "both named observation inputs compile a union");
    assert!(total > program.logical_regions().len());
    let bindings = vec![vec![]; plans.len()];
    let mut walk = crate::domain::shared_walk::types::prepared_walk::value::PreparedWalk::compile(
        &plans, &bindings,
    );
    walk.retain_program_consumers(&program, &bindings).unwrap();
    let census = walk.census();
    assert_eq!(census.instructions.0, total);
    assert_eq!(
        census.observation.0,
        unions + 2,
        "union heads and observation consumers"
    );
    assert_eq!(
        census.shared.0
            + census.representation.0
            + census.dependent.0
            + census.observation.0
            + census.row.0
            + census.derived.0
            + census.request.0
            + census.continuation.0,
        total,
    );
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
