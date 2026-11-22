//! The `splicing` service: free functions named by verb over the definitions of a linked program lowered into its
//! plan. An expansion of a fragment that binds no relation input is made of the fragment's definitions and the values its
//! call bound, whatever rule calls it; the steps its lowering makes are the same in every plan but for their places, the
//! places of the comparison origins they intern, the identities of their definitions and the call sites above the
//! expansion. So the first lowering of such an expansion is kept with those four taken out, and a later plan with the
//! same expansion takes its steps with its own put in, in place of lowering the definitions again.

use super::constants::{LOWERED_EXPANSIONS, LOWERING_ENVIRONMENTS};
use super::types::{
    DefinitionPlace, EnvironmentShelf, ExpansionShelf, KeptExpansion, LoweredExpansion,
    LoweredExpansions, LoweringEnvironment, SplicedLabel, SplicedOrigin, SplicedStep, TracePlace,
};
use crate::domain::defining::service::lower_definition;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{Builder, OriginCount, StepLabel};
use crate::domain::planning::service::{appended, next_step};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::hosted::types::HostedRelation;
use ryo_values::domain::observations::types::BoundObservationInput;
use ryo_values::domain::plan_parts::types::{ComparisonOrigin, Input, StepId};
use ryo_values::domain::plans::service::{
    comparisons_mut, comparisons_of, each_input, inputs_of, named,
};
use ryo_values::domain::program::service::identity_of;
use ryo_values::domain::program::types::{
    FragmentContract, LetStep, ParameterDeclaration, WrittenProgram,
};
use ryo_values::domain::query_tokens::types::{QueryOrigin, QueryTraceSite, StepIdentity};
use std::collections::HashMap;
use std::sync::{Arc, PoisonError};

/// Lowers every definition of a linked program into `builder`, in order: an expansion that binds no relation input
/// spliced from the shelf of this environment in `expansions` when it holds one, else lowered here and kept there; every
/// other definition lowered here.
pub(crate) fn lower_definitions(
    builder: &mut Builder<'_>,
    program: &WrittenProgram,
    expansions: &LoweredExpansions,
    catalog: &[Relation],
    hosted: &[HostedRelation],
    observations: &[BoundObservationInput],
) -> Result<(), LoweringError> {
    let shelf = shelf_for(
        expansions,
        catalog,
        hosted,
        observations,
        &program.parameters,
    );
    let closed = closed_expansions(program);
    let mut at = 0;
    while let Some(step) = program.steps.get(at) {
        if let Some(contract) = identity_of(step.name.1).and_then(|identity| closed.get(&identity))
            && builder.names.unidentified.is_empty()
            && let Some(block) = program
                .steps
                .get(at..at.saturating_add(contract.statements.0))
        {
            if let Some(lowered) = kept_lowering(&shelf, contract) {
                splice(builder, &lowered, block, contract);
            } else {
                let start = next_step(&builder.steps);
                let origins_before = OriginCount(builder.comparison_origins.len());
                for definition in block {
                    lower_definition(builder, program, definition)?;
                }
                if let Some(lowered) = recorded(builder, start, origins_before, block, contract) {
                    keep(&shelf, contract, lowered);
                }
            }
            at += block.len();
            continue;
        }
        lower_definition(builder, program, step)?;
        at += 1;
    }
    Ok(())
}

/// The shelf of the expansions lowered over this environment: the one kept for it, or a new one, kept from now on while
/// fewer than `LOWERING_ENVIRONMENTS` are.
pub(crate) fn shelf_for(
    expansions: &LoweredExpansions,
    catalog: &[Relation],
    hosted: &[HostedRelation],
    observations: &[BoundObservationInput],
    parameters: &[ParameterDeclaration],
) -> Arc<ExpansionShelf> {
    let mut held = expansions.0.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(kept) = held.iter().find(|kept| {
        kept.environment.catalog == catalog
            && kept.environment.hosted == hosted
            && kept.environment.observations == observations
            && kept.environment.parameters == parameters
    }) {
        return Arc::clone(&kept.shelf);
    }
    let shelf = Arc::new(ExpansionShelf::default());
    if held.len() < LOWERING_ENVIRONMENTS {
        held.push(EnvironmentShelf {
            environment: LoweringEnvironment {
                catalog: catalog.to_vec(),
                hosted: hosted.to_vec(),
                observations: observations.to_vec(),
                parameters: parameters.to_vec(),
            },
            shelf: Arc::clone(&shelf),
        });
    }
    shelf
}

/// The expansions of a linked program that bind no relation input, by the identity of their first definition: of the
/// expansions that start at one definition, the one with the most definitions, which holds the others.
pub(crate) fn closed_expansions(
    program: &WrittenProgram,
) -> HashMap<StepIdentity, &FragmentContract> {
    let mut closed: HashMap<StepIdentity, &FragmentContract> = HashMap::new();
    for contract in program
        .fragment_contracts
        .iter()
        .filter(|contract| contract.inputs.is_empty())
    {
        let held = closed.entry(contract.first).or_insert(contract);
        if contract.statements > held.statements {
            *held = contract;
        }
    }
    closed
}

/// The lowering kept on the shelf of the expansion `contract` names: the same fragment with the same values bound.
pub(crate) fn kept_lowering(
    shelf: &ExpansionShelf,
    contract: &FragmentContract,
) -> Option<Arc<LoweredExpansion>> {
    let held = shelf.0.lock().unwrap_or_else(PoisonError::into_inner);
    held.get(&contract.origin.source_id)?
        .iter()
        .find(|kept| kept.parameters == contract.parameters)
        .map(|kept| Arc::clone(&kept.lowered))
}

/// Keeps the lowering of the expansion `contract` names on the shelf, unless one is kept already or the shelf holds
/// `LOWERED_EXPANSIONS`.
pub(crate) fn keep(shelf: &ExpansionShelf, contract: &FragmentContract, lowered: LoweredExpansion) {
    let mut held = shelf.0.lock().unwrap_or_else(PoisonError::into_inner);
    if held.values().map(Vec::len).sum::<usize>() >= LOWERED_EXPANSIONS {
        return;
    }
    let kept = held.entry(contract.origin.source_id.clone()).or_default();
    if !kept
        .iter()
        .any(|kept| kept.parameters == contract.parameters)
    {
        kept.push(KeptExpansion {
            parameters: contract.parameters.clone(),
            lowered: Arc::new(lowered),
        });
    }
}

/// What the lowering of `block`, the definitions of the expansion `contract` names, made in `builder` from step `start`
/// and comparison origin `origins_before` on, with the places, identities and call sites of this plan taken out. None
/// when a step reads a step of the plan from before the expansion that is not a hosted relation's, names a step by a
/// name that is not one of the block's definitions, or a comparison origin is not below the expansion's call site: what
/// the lowering made then is not the expansion's alone.
pub(crate) fn recorded(
    builder: &Builder<'_>,
    start: StepId,
    origins_before: OriginCount,
    block: &[LetStep],
    contract: &FragmentContract,
) -> Option<LoweredExpansion> {
    let hosted = builder.hosted_steps.len();
    let definitions: HashMap<StepIdentity, usize> = block
        .iter()
        .enumerate()
        .filter_map(|(at, definition)| {
            identity_of(definition.name.1).map(|identity| (identity, at))
        })
        .collect();
    let first = usize::try_from(start.0).unwrap_or(usize::MAX);
    let mut steps = Vec::new();
    for (step, lowered) in builder
        .steps
        .get(first..)?
        .iter()
        .zip(builder.lowered.get(first..)?)
    {
        let reads_own = named(&inputs_of(&step.operation))
            .iter()
            .all(|input| match input {
                Input::Step(id) => {
                    id.0 >= start.0 || usize::try_from(id.0).is_ok_and(|id| id < hosted)
                }
                Input::Relation(_) | Input::BoundObservation(_) => true,
            });
        let interns_own = comparisons_of(&step.operation)
            .iter()
            .all(|comparison| match comparison.origin {
                ComparisonOrigin::Recorded(id) => {
                    usize::try_from(id.0).is_ok_and(|id| id >= origins_before.0)
                }
                ComparisonOrigin::Unrecorded => true,
            });
        if !reads_own || !interns_own {
            return None;
        }
        let label = match &lowered.name {
            StepLabel::Unnamed => SplicedLabel::Unnamed,
            StepLabel::Named(name) => {
                let at = *definitions.get(&identity_of(name.1)?)?;
                if block.get(at).map(|definition| &definition.name) != Some(name) {
                    return None;
                }
                SplicedLabel::Definition(DefinitionPlace(at))
            }
        };
        steps.push(SplicedStep {
            step: step.clone(),
            fields: lowered.fields.clone(),
            label,
        });
    }
    let base = &contract.origin.trace;
    let mut traces: Vec<Vec<QueryTraceSite>> = vec![Vec::new()];
    let mut origins = Vec::new();
    for origin in builder.comparison_origins.get(origins_before.0..)? {
        let below = origin.trace.strip_prefix(&base[..])?;
        let trace = TracePlace(
            if let Some(at) = traces.iter().position(|held| held.as_slice() == below) {
                at
            } else {
                traces.push(below.to_vec());
                traces.len() - 1
            },
        );
        origins.push(SplicedOrigin {
            source_id: origin.source_id.clone(),
            span: origin.span,
            trace,
        });
    }
    Some(LoweredExpansion {
        start,
        origins_before,
        steps,
        origins,
        traces,
    })
}

/// Puts the steps of `lowered` into `builder`'s plan, the lowering of `block`, the definitions of the expansion
/// `contract` names, with this plan's places, identities and call sites put in: the steps lowering `block` here makes.
pub(crate) fn splice(
    builder: &mut Builder<'_>,
    lowered: &LoweredExpansion,
    block: &[LetStep],
    contract: &FragmentContract,
) {
    let start = next_step(&builder.steps);
    let origins_before = u32::try_from(builder.comparison_origins.len()).unwrap_or(u32::MAX);
    let held_before = u32::try_from(lowered.origins_before.0).unwrap_or(u32::MAX);
    let traces: Vec<Arc<[QueryTraceSite]>> = lowered
        .traces
        .iter()
        .map(|below| {
            if below.is_empty() {
                Arc::clone(&contract.origin.trace)
            } else {
                contract.origin.trace.iter().chain(below).cloned().collect()
            }
        })
        .collect();
    for origin in &lowered.origins {
        builder.comparison_origins.push(QueryOrigin {
            source_id: origin.source_id.clone(),
            span: origin.span,
            trace: traces
                .get(origin.trace.0)
                .map_or_else(|| Arc::clone(&contract.origin.trace), Arc::clone),
        });
    }
    for spliced in &lowered.steps {
        let mut step = spliced.step.clone();
        each_input(&mut step.operation, &mut |input| {
            if let Input::Step(id) = input
                && id.0 >= lowered.start.0
            {
                id.0 = id.0 - lowered.start.0 + start.0;
            }
        });
        for comparison in comparisons_mut(&mut step.operation) {
            if let ComparisonOrigin::Recorded(id) = &mut comparison.origin {
                id.0 = id.0 - held_before + origins_before;
            }
        }
        let name = match spliced.label {
            SplicedLabel::Unnamed => None,
            SplicedLabel::Definition(at) => block.get(at.0).map(|definition| &definition.name),
        };
        appended(builder, name, step, spliced.fields.clone());
    }
}
