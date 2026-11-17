//! The `allocation_verdict` service: free functions named by verb that set the blocks each file allocated against the parser library's
//! and answer the most any of them allocated.

use std::collections::BTreeSet;

use ryo_values::domain::files::types::CorpusFile;

use super::constants::{ITEM_END, WARM_UP};
use super::types::{AllocatedBlocks, Differences, InputBytes, JobBlocks, ParserBlocks};

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

/// The most blocks any judged file allocated beyond the parser library: what the allocation measure holds to its best.
pub(crate) fn most(differences: &Differences) -> AllocatedBlocks {
    differences.0.iter().copied().max().unwrap_or_default()
}
