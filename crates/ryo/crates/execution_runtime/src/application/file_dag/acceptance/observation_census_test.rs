//! The shared walk's census counts every synthetic observation union a compiled program adds, beside the original
//! instructions, each with its observation barrier.
use super::observations_test::fixtures::{
    DeclaredFileProducers, Fixture, FrontierSlots, Names, fixture,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::{
    executable_program::{service::compile, types::InputSource},
    operating::errors::OperatingError,
    shared_walk::types::prepared_walk::value::PreparedWalk,
};
use ryo_values::domain::observations::types::{ObservationBindingName, ObservationColumnName};

fn declared(frontier: FrontierSlots) -> Fixture {
    fixture(
        frontier,
        Names {
            left: ObservationBindingName("left".into()),
            right: ObservationBindingName("right".into()),
            column: ObservationColumnName("file".into()),
        },
    )
}

fn the_census_includes_every_synthetic_observation_union() -> Result<(), OperatingError> {
    let Fixture {
        plans,
        graph,
        operators,
    } = declared(FrontierSlots::Declared);
    let slots = graph.observations.as_ref().unwrap().slots.clone();
    let program = compile(&plans, graph, &DeclaredFileProducers, &operators, ryo_execution_domain::domain::matching::types::PatternMatcher(&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl)).unwrap();
    use ryo_values::domain::{
        plan_parts::types::{Input as PlanInput, StepId},
        plans::types::Operation,
    };
    let mut observation_consumers = Vec::new();
    for (slot, declaration) in slots.iter().enumerate() {
        let plan = &plans[declaration.rule.0];
        assert!(
            matches!(plan.steps[0].operation, Operation::Select { from: PlanInput::BoundObservation(input), .. } if input == declaration.input)
        );
        let origin = program.rules()[declaration.rule.0].original_steps[0]
            .as_ref()
            .unwrap();
        let region = &program.regions()[program.logical_regions()[origin.logical.0].0];
        // These consumers remain individual heads, not fused continuations.
        assert_eq!(region.instructions().len(), 1);
        let head = &region.instructions()[0];
        assert_eq!(head.logical(), origin.logical);
        assert!(matches!(
            head.operation(),
            Operation::Select {
                from: PlanInput::Step(StepId(0)),
                ..
            }
        ));
        assert_eq!(
            &*region.inputs(),
            &[InputSource::Observation(
                ryo_execution_domain::domain::executable_program::types::ObservationPlace(slot)
            )]
        );
        observation_consumers.push(head);
    }
    assert_eq!(observation_consumers.len(), 2);
    for region in program
        .regions()
        .iter()
        .filter(|region| region.instructions()[0].logical().0 >= program.logical_regions().len())
    {
        assert_eq!(region.instructions().len(), 1);
        assert!(matches!(
            region.instructions()[0].operation(),
            Operation::Union { .. }
        ));
        assert!(
            region
                .inputs()
                .iter()
                .all(|source| matches!(source, InputSource::ObservationMember { .. }))
        );
    }
    let instructions = program
        .regions()
        .iter()
        .flat_map(|region| region.instructions().iter());
    let total = instructions.clone().count();
    let unions = instructions
        .filter(|instruction| instruction.logical().0 >= program.logical_regions().len())
        .count();
    assert_eq!(unions, 2, "both named observation inputs compile a union");
    assert!(total > program.logical_regions().len());
    let bindings = vec![vec![]; plans.len()];
    let mut walk = PreparedWalk::compile(&plans, &bindings);
    walk.retain_program_consumers(&program).unwrap();
    for head in &observation_consumers {
        assert!(
            walk.class_for(head).is_none(),
            "observation input is never a relation source walk"
        );
    }
    let census = walk.census();
    assert_eq!(census.instructions.0, total);
    assert_eq!(
        census.observation.0,
        unions + observation_consumers.len(),
        "synthetic union heads and normalized consumers keep their resolved observation barrier"
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

fn shared_walk_census_includes_every_synthetic_observation_union() -> Result<(), OperatingError> {
    let Fixture {
        plans,
        graph,
        operators,
    } = fixture(
        FrontierSlots::Declared,
        Names {
            left: ObservationBindingName("left".into()),
            right: ObservationBindingName("right".into()),
            column: ObservationColumnName("file".into()),
        },
    );
    let slots = graph.observations.as_ref().unwrap().slots.clone();
    let program = compile(&plans, graph, &DeclaredFileProducers, &operators, ryo_execution_domain::domain::matching::types::PatternMatcher(&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl)).unwrap();
    use ryo_values::domain::{
        plan_parts::types::{Input as PlanInput, StepId},
        plans::types::Operation,
    };
    let mut observation_consumers = Vec::new();
    for (slot, declaration) in slots.iter().enumerate() {
        let plan = &plans[declaration.rule.0];
        assert!(
            matches!(plan.steps[0].operation, Operation::Select { from: PlanInput::BoundObservation(input), .. } if input == declaration.input)
        );
        let origin = program.rules()[declaration.rule.0].original_steps[0]
            .as_ref()
            .unwrap();
        let region = &program.regions()[program.logical_regions()[origin.logical.0].0];
        // These consumers remain individual heads, not fused continuations.
        assert_eq!(region.instructions().len(), 1);
        let head = &region.instructions()[0];
        assert_eq!(head.logical(), origin.logical);
        assert!(matches!(
            head.operation(),
            Operation::Select {
                from: PlanInput::Step(StepId(0)),
                ..
            }
        ));
        assert_eq!(
            &*region.inputs(),
            &[InputSource::Observation(
                ryo_execution_domain::domain::executable_program::types::ObservationPlace(slot)
            )]
        );
        observation_consumers.push(head);
    }
    assert_eq!(observation_consumers.len(), 2);
    for region in program
        .regions()
        .iter()
        .filter(|region| region.instructions()[0].logical().0 >= program.logical_regions().len())
    {
        assert_eq!(region.instructions().len(), 1);
        assert!(matches!(
            region.instructions()[0].operation(),
            Operation::Union { .. }
        ));
        assert!(
            region
                .inputs()
                .iter()
                .all(|source| matches!(source, InputSource::ObservationMember { .. }))
        );
    }
    let instructions = program
        .regions()
        .iter()
        .flat_map(|region| region.instructions().iter());
    let total = instructions.clone().count();
    let unions = instructions
        .filter(|instruction| instruction.logical().0 >= program.logical_regions().len())
        .count();
    assert_eq!(unions, 2, "both named observation inputs compile a union");
    assert!(total > program.logical_regions().len());
    let bindings = vec![vec![]; plans.len()];
    let mut walk = PreparedWalk::compile(&plans, &bindings);
    walk.retain_program_consumers(&program).unwrap();
    for head in &observation_consumers {
        assert!(
            walk.class_for(head).is_none(),
            "observation input is never a relation source walk"
        );
    }
    let census = walk.census();
    assert_eq!(census.instructions.0, total);
    assert_eq!(
        census.observation.0,
        unions + observation_consumers.len(),
        "synthetic union heads and normalized consumers keep their resolved observation barrier"
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

type Input = fn() -> Result<(), OperatingError>;
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_census_includes_every_synthetic_observation_union"),
        word: Word::Succeeds,
        input: || the_census_includes_every_synthetic_observation_union,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("shared_walk_census_includes_every_synthetic_observation_union"),
        word: Word::Succeeds,
        input: || shared_walk_census_includes_every_synthetic_observation_union,
        expect: Expect::Value(()),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    input()
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
