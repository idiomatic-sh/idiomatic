//! The `answering` service: free functions named by verb over the answers of the cross-file steps, with no IO and no ports.

use super::types::{CrossSteps, Crossed, Todo};
use crate::domain::ordering::service::append;
use crate::domain::render::types::Rendered;
use crate::domain::workers::service::add_fed;
use ryo_execution::inbound::use_cases::execution::types::RowsFed;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_values::domain::run_metadata::types::{CrossSteps, RuleRunMetadata};
use ryo_values::domain::runs::types::RuleId;

/// Whether the plan has cross-file steps.
#[must_use]
pub(crate) fn cross_steps(plan: &Plan) -> CrossSteps {
    if plan
        .steps
        .iter()
        .any(|step| step.scope == StepScope::CrossFile)
    {
        CrossSteps::Has
    } else {
        CrossSteps::Lacks
    }
}

/// The rules whose cross-file steps are made, in the order of the rules.
#[must_use]
pub(crate) fn todo_list(metadata: &[RuleRunMetadata], ids: &[RuleId]) -> Vec<Todo> {
    metadata
        .iter()
        .enumerate()
        .filter(|(_, rule)| rule.has_cross_steps == CrossSteps::Present)
        .map(|(rule, _)| Todo {
            rule: RuleIndex(rule),
            id: MetadataUseCaseImpl.rule_id(ids, RuleIndex(rule)),
        })
        .collect()
}

/// Takes what the cross-file steps of a rule found into the findings that come after the files', and the rows each step was fed.
pub(crate) fn settle(
    todo: &Todo,
    crossed: Crossed,
    later: &mut Rendered,
    fed: &mut [Vec<RowsFed>],
    unknowns: &mut Vec<ryo_execution::inbound::use_cases::metadata::interface::Unknown>,
) -> Option<ryo_values::domain::records::types::CrossPublicationTransfer> {
    let Crossed {
        rendered: here,
        fed: more,
        observations,
        unknowns: mut incomplete,
    } = crossed;
    if let Some(slots) = fed.get_mut(todo.rule.0) {
        for (slot, each) in slots.iter_mut().zip(&more) {
            add_fed(slot, each);
        }
    }
    if let Some(here) = here {
        append(later, &here);
    }
    unknowns.append(&mut incomplete);
    observations
}
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::plan_parts::types::StepScope;

use ryo_execution::inbound::use_cases::metadata::{
    interface::MetadataUseCase, use_case::MetadataUseCaseImpl,
};
