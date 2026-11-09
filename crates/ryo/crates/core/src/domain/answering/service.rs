//! The `answering` service: free functions named by verb over the answers of the cross-file steps, with no IO and no ports.

use super::types::{CrossSteps, Crossed, Todo};
use crate::domain::ordering::service::append;
use crate::domain::render::types::Rendered;
use crate::domain::workers::types::RunDecisions;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_values::domain::run_metadata::types::{CrossFileSteps, RuleRunMetadata};
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
pub(crate) fn todo_list(
    decisions: &impl RunDecisions,
    metadata: &[RuleRunMetadata],
    ids: &[RuleId],
) -> Vec<Todo> {
    metadata
        .iter()
        .enumerate()
        .filter(|(_, rule)| rule.has_cross_steps == CrossFileSteps::Present)
        .map(|(rule, _)| Todo {
            rule: RuleIndex(rule),
            id: decisions.rule_id(ids, RuleIndex(rule)),
        })
        .collect()
}

/// Takes what the cross-file steps of a rule found into the findings that come after the files'.
pub(crate) fn settle(
    crossed: Crossed,
    later: &mut Rendered,
    unknowns: &mut Vec<ryo_execution::inbound::use_cases::metadata::interface::Unknown>,
) -> Option<ryo_values::domain::records::types::CrossPublicationTransfer> {
    let Crossed {
        rendered: here,
        observations,
        unknowns: mut incomplete,
    } = crossed;
    if let Some(here) = here {
        append(later, &here);
    }
    unknowns.append(&mut incomplete);
    observations
}
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::Plan;
