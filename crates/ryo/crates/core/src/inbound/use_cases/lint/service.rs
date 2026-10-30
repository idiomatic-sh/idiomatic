//! The `lint` service: local free-function helpers, private to this use case.

use super::constants::TASK_ANSWERED;
use super::errors::LintError;
use crate::domain::reading::service::buckets_of;
use crate::domain::storing::types::{HandedAgain, MadeAgain, Reuse};
use crate::outbound::lint_store::interface::ReadStore;
use ryo_values::domain::files::types::CorpusEntry;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::records::types::Record;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stores::types::{BucketsWanted, StoredHanded};
use std::sync::Arc;

/// What a task's slot holds once the workers are done: its answer.
///
/// # Panics
///
/// When no worker answered the task: the workers take tasks until none is left, so a slot left empty is a bug that must crash,
/// never a task quietly dropped from the answer.
#[expect(
    clippy::expect_used,
    reason = "the workers take tasks until none is left; a slot left empty is a bug that must crash, never a task dropped from the answer"
)]
pub(super) fn answered<T>(slot: Option<T>) -> T {
    slot.expect(TASK_ANSWERED)
}

/// The record kept of each of `files`, in their order; none for a file with no record.
pub(super) fn records_of<'a>(files: &[CorpusEntry], kept: &'a [Record]) -> Vec<Option<&'a Record>> {
    files
        .iter()
        .map(|file| {
            match kept.binary_search_by(|record| record.path.cmp(&file.path)) {
                Ok(at) => kept.get(at),
                // The search answers where the path would go: the file has no record.
                Err(_) => None,
            }
        })
        .collect()
}

/// The buckets a run that makes cross-file steps again reads of what the files handed them, those of the rules made again
/// alone, and what the store kept of them.
pub(super) fn handed_of(
    read: Option<&Arc<dyn ReadStore>>,
    plans: &[Plan],
    first_bucket: &[Bucket],
    again: &Reuse,
) -> Result<HandedAgain, LintError> {
    let wanted = match again {
        Reuse::Cross(MadeAgain::These(made)) => {
            BucketsWanted::These(buckets_of(plans, first_bucket, Some(made)))
        }
        Reuse::Cross(MadeAgain::All | MadeAgain::Nothing) | Reuse::Whole => BucketsWanted::All,
        Reuse::Again => BucketsWanted::These(Vec::new()),
    };
    let handed = match again {
        Reuse::Cross(_) => read
            .map(|read| read.handed(&wanted))
            .transpose()?
            .unwrap_or_default(),
        Reuse::Whole | Reuse::Again => StoredHanded::default(),
    };
    Ok(HandedAgain { wanted, handed })
}
