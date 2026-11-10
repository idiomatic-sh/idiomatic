//! Select an exact, nonempty subset only after the complete inventory is loaded.
use super::constants;
use super::types::ShardSelection;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::CaseSelection;
use ryo_benchmark_contracts::domain::workloads::types::{
    PreparedCase, case_count::value::CaseCount,
};

/// Partition the sorted inventory into this shard, then keep exactly the named cases.
///
/// # Errors
/// Refuses a selection that is empty or names a case outside the shard.
pub fn select(
    mut cases: Vec<PreparedCase>,
    selection: CaseSelection<'_>,
) -> Result<ShardSelection, Error> {
    cases.sort_by(|left, right| (&left.rule, &left.name).cmp(&(&right.rule, &right.name)));
    let global_count = cases.len();
    let assigned = match selection.shard {
        ryo_benchmark_contracts::domain::requests::types::ShardPartition::Shard(shard) => cases
            .into_iter()
            .enumerate()
            .filter_map(|(index, case)| {
                (index % shard.count().0 == shard.index().0).then_some(case)
            })
            .collect::<Vec<_>>(),
        ryo_benchmark_contracts::domain::requests::types::ShardPartition::All => cases,
    };
    let assigned_ids = assigned
        .iter()
        .map(|case| {
            ryo_benchmark_contracts::domain::requests::types::CaseId(format!(
                "{}/{}",
                case.rule, case.name
            ))
        })
        .collect::<std::collections::BTreeSet<_>>();
    let selected_cases = match selection.ids {
        ryo_benchmark_contracts::domain::requests::types::IdSelection::Subset(selected)
            if selected.is_empty() || !selected.is_subset(&assigned_ids) =>
        {
            return Err(Error::Invalid(
                constants::CASE_SELECTION_IS_EMPTY_OR_OUTSIDE.into(),
            ));
        }
        ryo_benchmark_contracts::domain::requests::types::IdSelection::Subset(selected) => assigned
            .into_iter()
            .filter(|case| {
                selected.contains(&ryo_benchmark_contracts::domain::requests::types::CaseId(
                    format!("{}/{}", case.rule, case.name),
                ))
            })
            .collect::<Vec<_>>(),
        ryo_benchmark_contracts::domain::requests::types::IdSelection::WholeShard => assigned,
    };
    Ok(ShardSelection {
        global_count: CaseCount(global_count),
        selected_cases,
    })
}
