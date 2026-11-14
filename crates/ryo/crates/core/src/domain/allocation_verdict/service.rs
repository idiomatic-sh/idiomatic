//! The `allocation_verdict` service: free functions named by verb that set the blocks each file allocated against the parser library's
//! and say whether they are within what the findings explain.

use std::collections::BTreeSet;

use ryo_values::domain::files::types::CorpusFile;

use super::constants::{GROWTHS_PER_DOUBLING, ITEM_END, MOST_BEYOND, WARM_UP};
use super::types::{
    AllocatedBlocks, AllocationVerdict, Ceiling, Differences, FileCount, InputBytes, JobBlocks,
    ParserBlocks, TotalBlocks,
};
use crate::domain::corpus_copies::types::CopyCount;

/// The blocks each job allocated beyond the parser library, for the file it read. The first file of each input grows what the worker
/// keeps for the rows of its relation, and so do the first files of all: neither is judged.
pub(crate) fn differences(
    jobs: &JobBlocks,
    files: &[CorpusFile],
    parser: &ParserBlocks,
) -> Differences {
    let mut seen = BTreeSet::new();
    let mut found = Vec::new();
    for (place, (blocks, file)) in jobs.0.iter().zip(files).enumerate() {
        let first = seen.insert(
            file.path
                .0
                .split_once(ITEM_END)
                .map_or(&*file.path.0, |(item, _)| item),
        );
        if !first && place >= WARM_UP {
            let alone = parser
                .0
                .get(&InputBytes(file.bytes.0.clone()))
                .copied()
                .unwrap_or_default();
            found.push(AllocatedBlocks(blocks.saturating_sub(alone.0)));
        }
    }
    Differences(found)
}

/// Whether the files, `copies` of every input, allocated no more than allowed: with no `ceiling`, the target, a block beyond the
/// parser library for a few files, logarithmic in their number, for the findings vector that doubles now and then; under a
/// ceiling, the same number of files judged, none above its most and all together not above its total.
pub(crate) fn verdict(
    differences: &Differences,
    copies: CopyCount,
    ceiling: Option<Ceiling>,
) -> AllocationVerdict {
    let Some(Ceiling {
        most,
        total,
        judged,
    }) = ceiling
    else {
        return target(differences, copies);
    };
    let found = FileCount(differences.0.len());
    if found != judged {
        return AllocationVerdict::OtherJudged { judged: found };
    }
    let highest = differences.0.iter().copied().max().unwrap_or_default();
    if highest > most {
        return AllocationVerdict::AboveMost { most: highest };
    }
    let summed = TotalBlocks(differences.0.iter().map(|each| each.0).sum());
    if summed > total {
        return AllocationVerdict::AboveTotal { total: summed };
    }
    AllocationVerdict::Within
}

/// The target's verdict: a block beyond the parser library for a few files, logarithmic in their number.
fn target(differences: &Differences, copies: CopyCount) -> AllocationVerdict {
    if differences.0.iter().any(|each| each.0 > MOST_BEYOND) {
        return AllocationVerdict::BeyondOneBlock;
    }
    let grew = differences.0.iter().filter(|each| each.0 > 0).count();
    let doublings = usize::try_from(copies.0.ilog2()).unwrap_or(usize::MAX);
    if grew <= GROWTHS_PER_DOUBLING.saturating_mul(doublings) {
        AllocationVerdict::Within
    } else {
        AllocationVerdict::TooManyGrew {
            grew: FileCount(grew),
            judged: FileCount(differences.0.len()),
        }
    }
}
