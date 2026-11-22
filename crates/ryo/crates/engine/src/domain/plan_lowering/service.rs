//! The `plan_lowering` service: a pack lowered to its finish plan, and one pass run on a plan.

use super::axes::service::existence_marked;
use super::counts::service::counted;
use super::expansion::service::expanded;
use super::semijoins::service::semijoined;
use super::types::{PackPrograms, PlanningStage};
use crate::domain::finish_plans::types::FinishPlan;

/// The pack's plan, through `stage`: its rules expanded, then each pass up to the stage in its order.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::plan_lowering::service::lowered;
/// use ryo_engine::domain::plan_lowering::types::{PackPrograms, PlanningStage};
/// use ryo_values::domain::program::types::ParsedModules;
///
/// let pack = PackPrograms { rules: Vec::new(), fragments: ParsedModules(Default::default()) };
/// assert!(lowered(&pack, PlanningStage::Axes).steps.is_empty());
/// ```
#[must_use]
pub fn lowered(pack: &PackPrograms, stage: PlanningStage) -> FinishPlan {
    let mut plan = expanded(pack);
    for pass in [
        PlanningStage::Semijoins,
        PlanningStage::Counts,
        PlanningStage::Axes,
    ] {
        if pass <= stage {
            let _marked = passed(&mut plan, pass);
        }
    }
    plan
}

/// Runs the one pass a stage adds on `plan`, as it stands: nothing for `Expanded`, which is no pass over a plan. Answers
/// how many steps the pass marked.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::types::FinishPlan;
/// use ryo_engine::domain::plan_lowering::service::passed;
/// use ryo_engine::domain::plan_lowering::types::PlanningStage;
///
/// let mut plan = FinishPlan { steps: Vec::new(), rules: Vec::new(), unplanned: Vec::new() };
/// assert_eq!(passed(&mut plan, PlanningStage::Semijoins), 0);
/// ```
pub fn passed(plan: &mut FinishPlan, stage: PlanningStage) -> usize {
    match stage {
        PlanningStage::Expanded => 0,
        PlanningStage::Semijoins => semijoined(plan),
        PlanningStage::Counts => counted(plan),
        PlanningStage::Axes => existence_marked(plan),
    }
}
