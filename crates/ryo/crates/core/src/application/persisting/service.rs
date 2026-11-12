//! What a run reads again of what the files handed the last, and what it keeps for the next, over the store it read beside its files.
use super::types::KeepRun;
use crate::domain::reading::service::buckets_of;
use crate::domain::render::types::Answering;
use crate::domain::storing::service::stored_of;
use crate::domain::storing::types::{HandedAgain, ToKeep};
use crate::outbound::lint_store::errors::LintStoreError;
use crate::outbound::lint_store::interface::ReadStore;
use crate::outbound::lint_store::types::StoreKeeping;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stores::types::{BucketsWanted, MadeAgain, ReadHanded, Reuse};
use std::sync::Arc;

/// The buckets a run that makes cross-file steps again reads of what the files handed them, those of the rules made again
/// alone, and what the store kept of them.
pub(crate) fn handed_of(
    read: Option<&Arc<dyn ReadStore>>,
    metadata: &[ryo_values::domain::run_metadata::types::RuleRunMetadata],
    first_bucket: &[Bucket],
    again: &Reuse,
    producer_buckets: &[Bucket],
) -> Result<HandedAgain, LintStoreError> {
    let mut wanted = match again {
        Reuse::Cross(MadeAgain::These(made)) => {
            BucketsWanted::These(buckets_of(metadata, first_bucket, Some(made)))
        }
        Reuse::Cross(MadeAgain::All | MadeAgain::Nothing) | Reuse::Whole => BucketsWanted::All,
        Reuse::Again => BucketsWanted::These(Vec::new()),
    };
    if let BucketsWanted::These(buckets) = &mut wanted {
        buckets.extend_from_slice(producer_buckets);
        buckets.sort_unstable();
        buckets.dedup();
    }
    let handed = match again {
        Reuse::Cross(_) => read
            .map(|read| read.handed(&wanted))
            .transpose()?
            .unwrap_or_default(),
        Reuse::Whole | Reuse::Again => ReadHanded::default(),
    };
    Ok(HandedAgain { wanted, handed })
}

/// Keeps what this run found for the next, when it keeps anything and it is not what the store already holds: a row for each
/// file listed, its findings, and those of the rules that read across files. `keeps` says whether the store keeps anything, and
/// `read` is the store the run read beside its files, when it read one.
pub(crate) fn kept(
    keeps: StoreKeeping,
    read: Option<&Arc<dyn ReadStore>>,
    run: &KeepRun<'_>,
    keeping: ToKeep<'_>,
) -> Result<(), LintStoreError> {
    if *run.answering != Answering::Findings || keeps == StoreKeeping::Discards {
        return Ok(());
    }
    // A run that used everything again and read no file keeps the store as it is.
    if *run.again == Reuse::Again && run.read_again.is_empty() {
        return Ok(());
    }
    // What the files handed but their words is made again from the records; the words of the buckets the run did not make
    // again are kept as they are.
    let rebuilt = match run.again {
        Reuse::Whole | Reuse::Cross(MadeAgain::All | MadeAgain::Nothing) => BucketsWanted::All,
        Reuse::Cross(MadeAgain::These(_)) | Reuse::Again => run.wanted.clone(),
    };
    let loaded;
    let last_handed = match run.again {
        Reuse::Cross(_) | Reuse::Whole => run.handed,
        Reuse::Again => {
            loaded = read
                .map(|read| read.handed(&BucketsWanted::These(Vec::new())))
                .transpose()?
                .unwrap_or_default();
            &loaded
        }
    };
    let unchanged: Vec<Bucket> = match &rebuilt {
        BucketsWanted::All => Vec::new(),
        BucketsWanted::These(rebuilt) => buckets_of(run.metadata, run.first_bucket, None)
            .into_iter()
            .filter(|bucket| rebuilt.binary_search(bucket).is_err())
            .collect(),
    };
    let to_store = stored_of(keeping, run.stored, last_handed, &rebuilt, unchanged);
    if let Some(read) = read {
        read.keep(to_store.stored, &to_store.kept)?;
    }
    Ok(())
}
