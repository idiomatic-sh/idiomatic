//! The `counts` service.

use crate::domain::finish_plans::arguments::groups::types::CountedRows;
use crate::domain::finish_plans::types::{
    CountMark, FinishPlan, FinishStep, ProbeMark, RuleEmit, SourceOrdinal,
};
use crate::domain::finish_plans::service::readers;
use crate::domain::plan_lowering::semijoins::service::kept_source;
use hashbrown::HashSet;

/// The plan with every select the pass admits marked to be counted; answers how many it marked.
pub(crate) fn counted(plan: &mut FinishPlan) -> usize {
    let read_by = readers(&plan.steps);
    let emitted: HashSet<usize> = plan
        .rules
        .iter()
        .filter_map(|rule| match &rule.emit {
            RuleEmit::Written(emit) => Some(emit.step.0),
            RuleEmit::Unwritten => None,
        })
        .collect();
    let marks: Vec<(usize, SourceOrdinal)> = plan
        .steps
        .iter()
        .enumerate()
        .filter_map(|(place, step)| {
            let FinishStep::Select(select) = step else {
                return None;
            };
            if emitted.contains(&place) {
                return None;
            }
            let kept = kept_source(select, false)?;
            let [reader] = read_by.get(place)?.as_slice() else {
                return None;
            };
            match plan.steps.get(reader.0)? {
                FinishStep::GroupCount(count)
                    if count.from.0 == place
                        && !matches!(&count.distinct, CountedRows::Distinct(column) if !column.0.is_empty()) =>
                {
                    Some((place, SourceOrdinal(1 - kept)))
                }
                _ => None,
            }
        })
        .collect();
    for (place, probed) in &marks {
        if let Some(FinishStep::Select(select)) = plan.steps.get_mut(*place) {
            select.probe = ProbeMark::Probed(*probed);
            select.count = CountMark::Counted;
        }
    }
    marks.len()
}
