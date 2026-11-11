//! The support of the `census` cases: a compiled program of two named observation inputs, read
//! back as what its consumers, unions and walk census answer.
use super::super::fixtures::{DeclaredFileProducers, Fixture, FrontierSlots, Names, fixture};
use ryo_execution_domain::domain::{
    executable_program::{
        service::compile,
        types::{InputSource, ObservationPlace},
    },
    matching::types::PatternMatcher,
    operating::errors::OperatingError,
    shared_walk::types::prepared_walk::value::PreparedWalk,
};
use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
use ryo_values::domain::observations::types::{ObservationBindingName, ObservationColumnName};
use ryo_values::domain::{
    plan_parts::types::{Input as PlanInput, StepId},
    plans::types::Operation,
};

/// The names a declared program binds its two observation inputs and their column with.
#[derive(Clone, Copy, Debug)]
pub(super) struct Declared {
    pub(super) left: &'static str,
    pub(super) right: &'static str,
    pub(super) column: &'static str,
}

/// The walk census's counts, one per route.
#[derive(Debug, PartialEq)]
pub(super) struct Routes {
    pub(super) instructions: usize,
    pub(super) shared: usize,
    pub(super) representation: usize,
    pub(super) dependent: usize,
    pub(super) observation: usize,
    pub(super) row: usize,
    pub(super) derived: usize,
    pub(super) request: usize,
    pub(super) continuation: usize,
}

/// What the compiled program and its walk answered.
#[derive(Debug, PartialEq)]
pub(super) struct Census {
    /// Every declared slot's rule selects its bound observation in its first step.
    pub(super) consumers_select_their_bound_observation: bool,
    /// Every consumer is a lone head, selecting step 0 of its region, which reads its slot.
    pub(super) consumers_are_lone_heads_over_their_slot: bool,
    /// How many observation consumers there are.
    pub(super) consumers: usize,
    /// Every synthetic region is one union over observation members.
    pub(super) unions_are_lone_heads_over_observation_members: bool,
    /// How many synthetic union instructions there are.
    pub(super) unions: usize,
    /// How many instructions the program holds in all.
    pub(super) instructions: usize,
    /// How many logical regions the program holds.
    pub(super) logical_regions: usize,
    /// No consumer is a relation source walk.
    pub(super) consumers_have_no_walk_class: bool,
    /// The walk's census.
    pub(super) routes: Routes,
}

/// Compiles the declared program, retains its consumers on the shared walk, and reads its census.
pub(super) fn census(declared: Declared) -> Result<Census, OperatingError> {
    let Fixture {
        plans,
        graph,
        operators,
    } = fixture(
        FrontierSlots::Declared,
        Names {
            left: ObservationBindingName(declared.left.into()),
            right: ObservationBindingName(declared.right.into()),
            column: ObservationColumnName(declared.column.into()),
        },
    );
    let slots = graph.observations.as_ref().unwrap().slots.clone();
    let program = compile(
        &plans,
        graph,
        &DeclaredFileProducers,
        &operators,
        PatternMatcher(&MatchingUseCaseImpl),
    )?;
    let mut consumers_select_their_bound_observation = true;
    let mut consumers_are_lone_heads_over_their_slot = true;
    let mut observation_consumers = Vec::new();
    for (slot, declaration) in slots.iter().enumerate() {
        let plan = &plans[declaration.rule.0];
        consumers_select_their_bound_observation &= matches!(plan.steps[0].operation, Operation::Select { from: PlanInput::BoundObservation(input), .. } if input == declaration.input);
        let origin = program.rules()[declaration.rule.0].original_steps[0]
            .as_ref()
            .unwrap();
        let region = &program.regions()[program.logical_regions()[origin.logical.0].0];
        // These consumers remain individual heads, not fused continuations.
        let head = &region.instructions()[0];
        consumers_are_lone_heads_over_their_slot &= region.instructions().len() == 1
            && head.logical() == origin.logical
            && matches!(
                head.operation(),
                Operation::Select {
                    from: PlanInput::Step(StepId(0)),
                    ..
                }
            )
            && *region.inputs() == [InputSource::Observation(ObservationPlace(slot))];
        observation_consumers.push(head);
    }
    let unions_are_lone_heads_over_observation_members = program
        .regions()
        .iter()
        .filter(|region| region.instructions()[0].logical().0 >= program.logical_regions().len())
        .all(|region| {
            region.instructions().len() == 1
                && matches!(
                    region.instructions()[0].operation(),
                    Operation::Union { .. }
                )
                && region
                    .inputs()
                    .iter()
                    .all(|source| matches!(source, InputSource::ObservationMember { .. }))
        });
    let instructions = program
        .regions()
        .iter()
        .flat_map(|region| region.instructions().iter());
    let total = instructions.clone().count();
    let unions = instructions
        .filter(|instruction| instruction.logical().0 >= program.logical_regions().len())
        .count();
    let bindings = vec![vec![]; plans.len()];
    let mut walk = PreparedWalk::compile(&plans, &bindings);
    walk.retain_program_consumers(&program)?;
    let consumers_have_no_walk_class = observation_consumers
        .iter()
        .all(|head| walk.class_for(head).is_none());
    let census = walk.census();
    Ok(Census {
        consumers_select_their_bound_observation,
        consumers_are_lone_heads_over_their_slot,
        consumers: observation_consumers.len(),
        unions_are_lone_heads_over_observation_members,
        unions,
        instructions: total,
        logical_regions: program.logical_regions().len(),
        consumers_have_no_walk_class,
        routes: Routes {
            instructions: census.instructions.0,
            shared: census.shared.0,
            representation: census.representation.0,
            dependent: census.dependent.0,
            observation: census.observation.0,
            row: census.row.0,
            derived: census.derived.0,
            request: census.request.0,
            continuation: census.continuation.0,
        },
    })
}
