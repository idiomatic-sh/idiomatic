//! Extract the few non-executable rule facts once while plans are transient.
use super::types::{
    CrossSteps, FeatureSlot, FeatureSource, FrontierCapture, ProjectRead, RuleRunMetadata,
    StepMetadata,
};
use crate::domain::plans::service::{comparisons_of, kind_of};
use crate::domain::{
    plan_parts::types::Input,
    plans::{
        service::{inputs_of, named},
        types::{Feature, Operation, Plan},
    },
};
use crate::domain::plan_parts::types::StepScope;
use ryo_relations::domain::relations::{service::provenance_of, types::Provenance};

#[must_use]
pub fn from_plans(plans: &[Plan]) -> Vec<RuleRunMetadata> {
    plans.iter().map(from_plan).collect()
}

fn from_plan(plan: &Plan) -> RuleRunMetadata {
    RuleRunMetadata {
        reads_project: if plan.frontier.iter().any(|input| {
            matches!(input, Input::Relation(relation) if provenance_of(*relation) == Provenance::Manifests)
        }) {
            ProjectRead::Reads
        } else {
            ProjectRead::Ignores
        },
        has_cross_steps: if plan.steps.iter().any(|step| step.scope == StepScope::CrossFile) {
            CrossSteps::Present
        } else {
            CrossSteps::Absent
        },
        frontier_capture: plan.frontier.iter().map(|input| {
            if matches!(input, Input::BoundObservation(id) if plan.observation_inputs
                .get(id.0 as usize)
                .is_some_and(|schema| schema.scope == StepScope::CrossFile))
            {
                FrontierCapture::CrossFile
            } else {
                FrontierCapture::Captured
            }
        }).collect(),
        steps: plan.steps.iter().map(|step| {
            let mut features = vec![FeatureSlot {
                feature: Feature::Operation(kind_of(&step.operation)),
                source: FeatureSource::AllInputs,
            }];
            features.extend(comparisons_of(&step.operation).iter().map(|comparison| FeatureSlot {
                feature: Feature::Family(comparison.test),
                source: FeatureSource::AllInputs,
            }));
            if let Operation::Axis { direction, .. } = &step.operation {
                features.push(FeatureSlot {
                    feature: Feature::Direction(*direction),
                    source: FeatureSource::AllInputs,
                });
            }
            let inputs = inputs_of(&step.operation);
            features.extend(named(&inputs).iter().enumerate().filter_map(|(place, input)| {
                let Input::Relation(relation) = input else { return None; };
                Some(FeatureSlot {
                    feature: Feature::Relation(*relation),
                    source: FeatureSource::Input(place),
                })
            }));
            StepMetadata {
                operation: kind_of(&step.operation),
                scope: step.scope,
                features: features.into_boxed_slice(),
            }
        }).collect(),
    }
}
