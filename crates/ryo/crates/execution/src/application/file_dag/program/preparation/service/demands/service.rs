//! What each rule asks of its file and which outputs each frontier commits.
use crate::application::file_dag::program::preparation::types::{
    ObservationMetadata, RuleMetadata,
};
use crate::domain::coverage::types::RuleIndex;
use crate::domain::executable_program::file_demand::types::{
    FileRuleDemand, ObservationDemandSource,
};
use crate::domain::executable_program::types::executable_program::value::ExecutableProgram;
use crate::domain::operating::errors::OperatingError;
use ryo_values::domain::records::types::ObservationIndex;

/// One demand per rule, in rule order.
pub(in crate::application::file_dag::program::preparation) fn file_demands(
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
            crate::domain::executable_program::file_demand::service::compile(
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
pub(in crate::application::file_dag::program::preparation) fn frontier_outputs(
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
