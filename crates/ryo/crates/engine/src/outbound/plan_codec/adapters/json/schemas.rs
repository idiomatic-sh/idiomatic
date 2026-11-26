//! The plan as JSON: its nodes, each rule's emit (`null` for a rule whose query writes none), and each rule that did not
//! lower with why.

use super::nodes::schemas::NodeSchema;
use super::ordered::schemas::OrderedMap;
use ryo_values::domain::finish_plans::arguments::types::StepNumber;
use ryo_values::domain::finish_plans::types::{
    EmitSubject, FinishPlan, FinishStep, PlanEmit, PlannedRule, RuleEmit, UnplannedReason,
    UnplannedRule,
};
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::runs::types::RuleId;
use serde::{Deserialize, Serialize};

/// A plan.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct PlanSchema {
    /// The steps.
    nodes: Vec<NodeSchema>,
    /// Each rule's emit.
    emits: OrderedMap<Option<EmitSchema>>,
    /// Each rule that did not lower, and why.
    failed: OrderedMap<String>,
}

/// A rule's emit.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct EmitSchema {
    /// The step.
    node: usize,
    /// What a finding is about.
    subject: String,
    /// The key columns.
    key: Vec<String>,
}

impl From<&FinishPlan> for PlanSchema {
    fn from(plan: &FinishPlan) -> Self {
        Self {
            nodes: plan.steps.iter().map(NodeSchema::from).collect(),
            emits: OrderedMap(
                plan.rules
                    .iter()
                    .map(|rule| {
                        let emit = match &rule.emit {
                            RuleEmit::Unwritten => None,
                            RuleEmit::Written(emit) => Some(EmitSchema {
                                node: emit.step.0,
                                subject: emit.subject.0.to_string(),
                                key: emit.key.iter().map(|key| key.0.to_string()).collect(),
                            }),
                        };
                        (rule.rule.0.to_string(), emit)
                    })
                    .collect(),
            ),
            failed: OrderedMap(
                plan.unplanned
                    .iter()
                    .map(|rule| (rule.rule.0.to_string(), rule.reason.0.to_string()))
                    .collect(),
            ),
        }
    }
}

impl From<PlanSchema> for FinishPlan {
    fn from(plan: PlanSchema) -> Self {
        Self {
            steps: plan.nodes.into_iter().map(FinishStep::from).collect(),
            rules: plan
                .emits
                .0
                .into_iter()
                .map(|(rule, emit)| PlannedRule {
                    rule: RuleId(rule.into()),
                    emit: emit.map_or(RuleEmit::Unwritten, |emit| {
                        RuleEmit::Written(PlanEmit {
                            step: StepNumber(emit.node),
                            subject: EmitSubject(emit.subject.into()),
                            key: emit.key.into_iter().map(|key| ColumnName(key.into())).collect(),
                        })
                    }),
                })
                .collect(),
            unplanned: plan
                .failed
                .0
                .into_iter()
                .map(|(rule, reason)| UnplannedRule {
                    rule: RuleId(rule.into()),
                    reason: UnplannedReason(reason.into()),
                    ungiven: None,
                })
                .collect(),
        }
    }
}
