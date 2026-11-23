//! The `plan_lowering` service: a pack lowered to its finish plan, and one pass run on a plan.

use super::axes::service::existence_marked;
use super::counts::service::counted;
use super::expansion::service::expanded;
use super::semijoins::service::semijoined;
use super::readers::service::step_uses;
use super::readers::types::Uses;
use super::types::{PackPrograms, PlanningStage};
use ryo_values::domain::finish_plans::types::FinishPlan;

/// The pack's plan, through `stage`: its rules expanded, then each pass up to the stage in its order.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::plan_lowering::service::lowered;
/// use ryo_engine::domain::plan_lowering::types::{PackPrograms, PlanningStage};
/// use ryo_values::domain::program::types::ParsedModules;
///
/// let pack = PackPrograms { rules: Vec::new(), fragments: ParsedModules(Default::default()), context: Default::default() };
/// assert!(lowered(&pack, PlanningStage::Axes).steps.is_empty());
/// ```
#[must_use]
pub fn lowered(pack: &PackPrograms, stage: PlanningStage) -> FinishPlan {
    let mut plan = expanded(pack);
    mark_through(&mut plan, stage);
    plan
}

/// Runs all marking passes through the requested stage on an expanded plan.
pub fn mark_through(plan: &mut FinishPlan, stage: PlanningStage) {
    if stage == PlanningStage::Expanded {
        return;
    }
    let uses = step_uses(plan);
    for pass in [
        PlanningStage::Semijoins,
        PlanningStage::Counts,
        PlanningStage::Axes,
    ] {
        if pass <= stage {
            let _marked = mark(plan, pass, &uses);
        }
    }
}

/// Runs the one pass a stage adds on `plan`, as it stands: nothing for `Expanded`, which is no pass over a plan. Answers
/// how many steps the pass marked.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::types::FinishPlan;
/// use ryo_engine::domain::plan_lowering::service::passed;
/// use ryo_engine::domain::plan_lowering::types::PlanningStage;
///
/// let mut plan = FinishPlan { steps: Vec::new(), rules: Vec::new(), unplanned: Vec::new() };
/// assert_eq!(passed(&mut plan, PlanningStage::Semijoins), 0);
/// ```
pub fn passed(plan: &mut FinishPlan, stage: PlanningStage) -> usize {
    if stage == PlanningStage::Expanded {
        return 0;
    }
    let uses = step_uses(plan);
    mark(plan, stage, &uses)
}

fn mark(plan: &mut FinishPlan, stage: PlanningStage, uses: &Uses) -> usize {
    match stage {
        PlanningStage::Expanded => 0,
        PlanningStage::Semijoins => semijoined(plan, uses),
        PlanningStage::Counts => counted(plan, uses),
        PlanningStage::Axes => existence_marked(plan, uses),
    }
}
