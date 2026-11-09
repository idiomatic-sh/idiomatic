//! What each rule asks of its file and which outputs each frontier commits.
use crate::application::file_dag::program::preparation::types::{
    ObservationMetadata, RuleMetadata,
};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::file_demand::types::{
    FileRuleDemand, ObservationDemandSource,
};
use ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::domain::records::types::ObservationIndex;

/// One demand per rule, in rule order.
pub(crate) fn file_demands(
    program: &ExecutableProgram,
    metadata: &[RuleMetadata],
) -> Result<Box<[FileRuleDemand]>, OperatingError> {
    Ok(metadata
        .iter()
        .enumerate()
        .map(|(rule, metadata)| {
            let outputs = metadata
                .observations
                .iter()
                .map(|root| ObservationDemandSource {
                    source: root.source.clone(),
                    phase: root.phase,
                })
                .collect::<Vec<_>>();
            ryo_execution_domain::domain::executable_program::file_demand::service::compile(
                program,
                RuleIndex(rule),
                &outputs,
                &metadata.observation_frontiers,
            )
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_boxed_slice())
}
/// For each rule and frontier input, the named outputs that the input commits.
pub(crate) fn frontier_outputs(
    metadata: &[RuleMetadata],
    named: Option<&ObservationMetadata>,
) -> Box<[Box<[Box<[ObservationIndex]>]>]> {
    metadata
        .iter()
        .enumerate()
        .map(|(rule, metadata)| {
            (0..metadata.frontiers.len())
                .map(|input| {
                    named
                        .iter()
                        .flat_map(|named| named.frontiers.iter())
                        .filter(|root| root.producer.0 == rule && root.frontier.0 == input)
                        .map(|root| root.output)
                        .collect::<Vec<_>>()
                        .into_boxed_slice()
                })
                .collect::<Vec<_>>()
                .into_boxed_slice()
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

/// What each rule needs of the derived views, in rule order: the relations its per-file regions read, each once in the order
/// first read, from their inputs and their operators' derived slots, and whether one of its steps walks a graph (an axis or a
/// reaching join).
pub(crate) fn derived_needs(
    program: &ExecutableProgram,
) -> Result<
    Box<[ryo_execution_store::application::file_dag::derived::prepared::RuleDerivedNeeds]>,
    OperatingError,
> {
    let mut needs = Vec::with_capacity(program.rules().len());
    for mapping in program.rules() {
        let mut reads = Vec::new();
        let mut graph = false;
        for origin in mapping.original_steps.iter() {
            let Ok(origin) = origin else {
                continue;
            };
            let region_id = *program
                .logical_regions()
                .get(origin.logical.0)
                .ok_or(OperatingError::NotRun)?;
            let region = program
                .regions()
                .get(region_id.0)
                .ok_or(OperatingError::NotRun)?;
            if region.phase() != ryo_values::domain::plan_parts::types::StepScope::PerFile {
                continue;
            }

            for source in region.inputs().iter() {
                if let ryo_execution_domain::domain::executable_program::types::InputSource::Relation(
                    relation,
                ) = source
                {
                    note_read(&mut reads, *relation);
                }
            }
            let instruction = region
                .instructions()
                .iter()
                .find(|instruction| instruction.logical() == origin.logical)
                .ok_or(OperatingError::NotRun)?;
            for group in instruction.contract().derived.slots.iter().flatten() {
                for use_ in std::iter::once(&group.primary).chain(group.secondary.iter()) {
                    for relation in use_.relations.iter().flatten() {
                        note_read(&mut reads, *relation);
                    }
                }
            }
            graph |= matches!(
                instruction.operation(),
                ryo_values::domain::plans::types::Operation::Axis { .. }
                    | ryo_values::domain::plans::types::Operation::Join { reach: Some(_), .. }
            );
        }
        needs.push(
            ryo_execution_store::application::file_dag::derived::prepared::RuleDerivedNeeds {
                relations: reads.into_boxed_slice(),
                graph: if graph {
                    ryo_execution_store::application::file_dag::derived::owner::cache::types::GraphAdmission::Enabled
                } else {
                    ryo_execution_store::application::file_dag::derived::owner::cache::types::GraphAdmission::Disabled
                },
            },
        );
    }
    Ok(needs.into_boxed_slice())
}

/// A relation a rule reads is recorded once, in the order it is first read.
fn note_read(
    reads: &mut Vec<ryo_relations::domain::relations::types::Relation>,
    relation: ryo_relations::domain::relations::types::Relation,
) {
    if !reads.contains(&relation) {
        reads.push(relation);
    }
}
