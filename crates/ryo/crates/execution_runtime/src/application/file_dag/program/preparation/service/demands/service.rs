//! What each rule asks of its file and which outputs each frontier commits.
use crate::application::file_dag::program::preparation::types::{
    ObservationMetadata, RuleMetadata,
};
use ryo_values::domain::records::types::ObservationIndex;

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
