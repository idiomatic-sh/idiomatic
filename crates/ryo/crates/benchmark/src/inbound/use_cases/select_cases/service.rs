//! Each flag is read once into the typed choice it names.
use super::constants::{
    CASE_RECEIPT_DESTINATION_MUST_BE_N, CASE_SELECTION, CASE_SELECTION_MUST_BE_NONEMPTY_AN,
    DUPLICATE_CASE_SELECTION_ARGUMENT, DUPLICATE_CASE_SHARD_ARGUMENT, EXPECTED_SHARD_INDEX_COUNT,
    INVALID_CASE_SHARD_COUNT, INVALID_CASE_SHARD_INDEX, SHARD, SHARD_SEPARATOR,
    UNEXPECTED_VERIFY_CASES_ARGUMENT,
};
use super::types::{CaseArgument, CaseRun, SelectedIds};
use crate::outbound::receipts::types::CaseReceipts;
use crate::outbound::suite::{interface::SuitePort, types::ArtifactPath};
use ryo_benchmark_contracts::domain::receipts::constants::CASE_RECEIPTS_ARGUMENT;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::{
    CaseId, ShardPartition,
    case_shard::value::{CaseShard, ShardCount, ShardIndex},
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Read every flag of one verification run.
pub(super) fn select(suite: &dyn SuitePort, arguments: &[CaseArgument]) -> Result<CaseRun, Error> {
    let mut case_receipts = None;
    let mut shard = ShardPartition::All;
    let mut case_selection = None;
    for argument in arguments {
        if let Some(path) = argument.0.strip_prefix(CASE_RECEIPTS_ARGUMENT) {
            if path.is_empty() || case_receipts.replace(PathBuf::from(path)).is_some() {
                return Err(Error::Invalid(CASE_RECEIPT_DESTINATION_MUST_BE_N.into()));
            }
        } else if argument.0.starts_with(SHARD) {
            if !matches!(shard, ShardPartition::All) {
                return Err(Error::Invalid(DUPLICATE_CASE_SHARD_ARGUMENT.into()));
            }
            shard = ShardPartition::Shard(parse_shard(argument)?);
        } else if let Some(path) = argument.0.strip_prefix(CASE_SELECTION) {
            if case_selection.is_some() {
                return Err(Error::Invalid(DUPLICATE_CASE_SELECTION_ARGUMENT.into()));
            }
            case_selection = Some(selected_ids(suite, Path::new(path))?);
        } else {
            return Err(Error::Invalid(UNEXPECTED_VERIFY_CASES_ARGUMENT.into()));
        }
    }
    Ok(CaseRun {
        receipts: case_receipts.map_or(CaseReceipts::Unrecorded, CaseReceipts::Recorded),
        shard,
        ids: case_selection.map_or(SelectedIds::WholeShard, SelectedIds::Subset),
    })
}

/// The case IDs a selection file lists, each exactly once.
fn selected_ids(suite: &dyn SuitePort, file: &Path) -> Result<BTreeSet<CaseId>, Error> {
    let selected = suite.selection(ArtifactPath(file))?;
    let selected_count = selected.len();
    let selected = selected
        .into_iter()
        .map(|name| CaseId(name.0))
        .collect::<BTreeSet<_>>();
    if selected.is_empty() || selected.len() != selected_count {
        return Err(Error::Invalid(CASE_SELECTION_MUST_BE_NONEMPTY_AN.into()));
    }
    Ok(selected)
}

/// Parse explicit zero-based shard coordinates using the validated application type.
fn parse_shard(argument: &CaseArgument) -> Result<CaseShard, Error> {
    let coordinates = argument
        .0
        .strip_prefix(SHARD)
        .ok_or_else(|| Error::Invalid(EXPECTED_SHARD_INDEX_COUNT.into()))?;
    let (index, count) = coordinates
        .split_once(SHARD_SEPARATOR)
        .ok_or_else(|| Error::Invalid(EXPECTED_SHARD_INDEX_COUNT.into()))?;
    let index = index
        .parse::<usize>()
        .map_err(|_invalid_index| Error::Invalid(INVALID_CASE_SHARD_INDEX.into()))?;
    let count = count
        .parse::<usize>()
        .map_err(|_invalid_count| Error::Invalid(INVALID_CASE_SHARD_COUNT.into()))?;
    CaseShard::new(ShardIndex(index), ShardCount(count))
}
