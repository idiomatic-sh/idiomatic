//! Expand existing warm request work before selecting persisted frontier buckets.
use crate::domain::coverage::types::RuleIndex;
use crate::domain::{
    file_program::{
        service::observations::replay_closure,
        types::ObservationRequest,
    },
    storing::types::{MadeAgain, Reuse},
};
use ryo_values::domain::{plans::types::Plan, records::types::PlacedRule};

pub(crate) fn observation_closure(
    again: Reuse,
    plans: &[Plan],
    request_graph: &std::sync::Arc<crate::domain::file_program::types::RequestGraph>,
    observations: ObservationRequest<'_>,
    first_bucket: &[ryo_values::domain::stored_facts::types::Bucket],
) -> Result<
    (Reuse, Vec<ryo_values::domain::stored_facts::types::Bucket>),
    crate::application::rule_compilation::errors::RuleCompilationError,
> {
    if matches!(observations, ObservationRequest::Independent) {
        return Ok((again, Vec::new()));
    }
    let Reuse::Cross(MadeAgain::These(selected)) = again else {
        return Ok((again, Vec::new()));
    };
    let graph = request_graph
        .observations
        .as_ref()
        .ok_or(crate::domain::file_program::errors::ObservationError::InvalidReference)?;
    let selected = selected
        .iter()
        .map(|rule| {
            usize::try_from(rule.0).map(RuleIndex).map_err(|_| {
                crate::domain::file_program::errors::ObservationError::InvalidReference
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let closed = replay_closure(graph, &selected)?;
    let mut needed = vec![false; plans.len()];
    for rule in &closed {
        needed[rule.0] = true;
    }
    let mut buckets = std::collections::BTreeSet::new();
    for root in &graph.frontiers {
        if graph.outputs[root.producer.0][root.output.0 as usize]
            .consumers
            .iter()
            .any(|consumer| needed[*consumer])
        {
            let first = *first_bucket
                .get(root.producer.0)
                .ok_or(crate::domain::file_program::errors::ObservationError::InvalidReference)?;
            buckets.insert(crate::domain::reading::service::frontier_bucket(
                first,
                root.frontier,
            ));
        }
    }
    let selected = closed
        .into_iter()
        .map(|rule| {
            u32::try_from(rule.0).map(PlacedRule).map_err(|_| {
                crate::domain::file_program::errors::ObservationError::InvalidReference
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((
        Reuse::Cross(MadeAgain::These(selected)),
        buckets.into_iter().collect(),
    ))
}
