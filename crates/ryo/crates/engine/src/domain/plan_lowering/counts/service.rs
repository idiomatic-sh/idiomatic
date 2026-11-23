//! The `counts` service.

use crate::domain::plan_lowering::readers::types::{Emitted, Uses};
use ryo_values::domain::finish_plans::arguments::groups::types::CountedRows;
use ryo_values::domain::finish_plans::types::{
    CountMark, FinishPlan, FinishStep, ProbeMark, SourceOrdinal,
};
use crate::domain::plan_lowering::semijoins::service::kept_source;

/// The plan with every select the pass admits marked to be counted; answers how many it marked.
pub(crate) fn counted(
    plan: &mut FinishPlan,
    uses: &Uses,
) -> usize {
    let marks: Vec<(usize, SourceOrdinal)> = plan
        .steps
        .iter()
        .enumerate()
        .filter_map(|(place, step)| {
            let FinishStep::Select(select) = step else {
                return None;
            };
            if uses.emitted[place] == Emitted::Yes {
                return None;
            }
            let [reader] = uses.readers.get(place)?.as_slice() else {
                return None;
            };
            match plan.steps.get(reader.0)? {
                FinishStep::GroupCount(count)
                    if count.from.0 == place
                        && !matches!(&count.distinct, CountedRows::Distinct(column) if !column.0.is_empty()) =>
                {
                    Some((place, SourceOrdinal(1 - kept_source(select,false)?)))
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
