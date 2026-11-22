//! The `axes` service.

use crate::domain::finish_plans::arguments::types::StepNumber;
use crate::domain::finish_plans::types::{ExistsMark, FinishPlan, FinishStep};
use crate::domain::plan_lowering::readers::service::{blind, plan_readers, used};
use crate::domain::plan_lowering::readers::types::Worked;
use ryo_values::domain::plan_parts::types::Direction;

/// The column an axis writes each start's relative to.
const RELATED: &str = "related";

/// The plan with every axis the pass admits marked `exists`; answers how many it marked.
pub(crate) fn existence_marked(plan: &mut FinishPlan) -> usize {
    let marks: Vec<usize> = {
        let readers = plan_readers(plan);
        let mut blind_memo = vec![Worked::Open; plan.steps.len()];
        let mut used_memo = vec![Worked::Open; plan.steps.len()];
        plan.steps
            .iter()
            .enumerate()
            .filter_map(|(place, step)| {
                let FinishStep::Axis(axis) = step else {
                    return None;
                };
                let direction = axis.arguments.direction;
                if !matches!(
                    direction,
                    Direction::Ancestor | Direction::Descendant | Direction::NearestAncestor
                ) {
                    return None;
                }
                let number = StepNumber(place);
                if used(&readers, &mut used_memo, number, RELATED) {
                    return None;
                }
                let admitted = blind(&readers, &mut blind_memo, number)
                    || direction == Direction::NearestAncestor;
                admitted.then_some(place)
            })
            .collect()
    };
    for place in &marks {
        if let Some(FinishStep::Axis(axis)) = plan.steps.get_mut(*place) {
            axis.exists = ExistsMark::Marked;
        }
    }
    marks.len()
}
