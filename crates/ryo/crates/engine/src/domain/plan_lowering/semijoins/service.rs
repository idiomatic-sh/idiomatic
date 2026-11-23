//! The `semijoins` service.

use ryo_values::domain::finish_plans::arguments::types::StepNumber;
use ryo_values::domain::finish_plans::types::{
    FinishPlan, FinishStep, ProbeMark, RowCondition, SelectStep, SelectedColumns, SourceOrdinal,
};
use crate::domain::plan_lowering::readers::service::{
    aliases_read, blind, conjuncts, plan_readers, source_alias,
};
use crate::domain::plan_lowering::readers::types::{ReadAlias, Worked};
use hashbrown::HashSet;

/// The plan with every select the pass admits marked a semijoin; answers how many it marked.
pub(crate) fn semijoined(plan: &mut FinishPlan) -> usize {
    let marks: Vec<(usize, SourceOrdinal)> = {
        let readers = plan_readers(plan);
        let mut memo = vec![Worked::Open; plan.steps.len()];
        plan.steps
            .iter()
            .enumerate()
            .filter_map(|(place, step)| match step {
                FinishStep::Select(select) => kept_source(select, true)
                    .filter(|_| blind(&readers, &mut memo, StepNumber(place)))
                    .map(|kept| (place, SourceOrdinal(1 - kept))),
                _ => None,
            })
            .collect()
    };
    for (place, probed) in &marks {
        if let Some(FinishStep::Select(select)) = plan.steps.get_mut(*place) {
            select.probe = ProbeMark::Probed(*probed);
        }
    }
    marks.len()
}

/// The place of the source a two-source select keeps: the one source its items read, when both sources have distinct
/// aliases and (with `tested`) one of its tests reads both.
pub(crate) fn kept_source(select: &SelectStep, tested: bool) -> Option<usize> {
    let SelectedColumns::Items(items) = &select.columns else {
        return None;
    };
    let [first, second] = select.sources.as_slice() else {
        return None;
    };
    let aliases = [source_alias(&first.alias), source_alias(&second.alias)];
    if aliases.contains(&ReadAlias::Unaliased) || aliases[0] == aliases[1] {
        return None;
    }
    let mut read: HashSet<ReadAlias> = HashSet::new();
    for item in items {
        read.extend(aliases_read(&item.value));
    }
    if tested {
        let both = match &select.condition {
            RowCondition::Every => false,
            RowCondition::Tested(condition) => conjuncts(condition).into_iter().any(|test| {
                let reads = aliases_read(test);
                aliases.iter().all(|alias| reads.contains(alias))
            }),
        };
        if !both {
            return None;
        }
    }
    let mut only = read.into_iter();
    match (only.next(), only.next()) {
        (Some(alias), None) => aliases.iter().position(|candidate| *candidate == alias),
        _ => None,
    }
}
