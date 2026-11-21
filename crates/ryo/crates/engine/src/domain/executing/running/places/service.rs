//! The services of the `places` part: what the walk's two kinds of word share. Whether the executor runs a test; the
//! table a site's index is over; what a body record reaches.

use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::records::service::body_at;
use crate::domain::code::records::types::BodyRecord;
use crate::domain::code::types::{IndexSite, PoolAt, ProgramCode, WordSlot};
use crate::domain::executing::running::types::WalkPlace;
use crate::domain::filled::types::FillRuns;
use crate::domain::made_tables::reading::service::{site_at, site_source};
use crate::domain::made_tables::types::ReadSource;
use crate::domain::stages::calling::service::call_runs;
use crate::domain::stages::testing::service::test_runs;
use crate::domain::steps::types::{CallMeaning, TestOperation};

/// Whether the executor runs a test: the fill side's answer (`stages::testing::service::test_runs`), which a `Test` word
/// of the loop shares, since its comparisons of cells and its tests of texts are the fill side's kernels.
pub(crate) const fn tested(test: Option<TestOperation>) -> FillRuns {
    match test {
        Some(test) => test_runs(test),
        None => FillRuns::NotYet,
    }
}

/// Whether the executor runs a call of a per-row meaning: the fill side's answer
/// (`stages::calling::service::call_runs`), which a `Call` word of the loop shares, since it is the same kernel.
pub(crate) const fn called(meaning: Option<CallMeaning>) -> FillRuns {
    match meaning {
        Some(meaning) => call_runs(meaning),
        None => FillRuns::NotYet,
    }
}

/// Whether the fill side reads a site's index, and the table the index is over when its source is a made table. An
/// index over a table read while its fill writes it, and an ordered index over the node rows, it does not build yet.
pub(crate) fn site_table(
    code: &ProgramCode,
    site: IndexSite,
    out: &mut Vec<WalkPlace>,
) -> FillRuns {
    let Ok(index) = site_at(code, site) else {
        return FillRuns::NotYet;
    };
    match site_source(code, index) {
        Ok(ReadSource::Made(table)) => {
            out.push(WalkPlace::Table(table));
            FillRuns::Runs
        }
        Ok(ReadSource::Nodes(_)) if index.ordered != NO_SLOT => FillRuns::NotYet,
        Ok(ReadSource::Nodes(_) | ReadSource::Filled(_)) => FillRuns::Runs,
        Ok(ReadSource::Written(_)) | Err(_) => FillRuns::NotYet,
    }
}

/// What a body record reaches: an inline body its program in the asking unit, a pattern in call form its program in its
/// own unit, a pattern in table form its table. Answers the place after the record; none when the pool does not hold it
/// or the fill side does not read the table-form pattern's index.
pub(crate) fn body(
    code: &ProgramCode,
    at: PoolAt,
    unit: WordSlot,
    out: &mut Vec<WalkPlace>,
) -> Option<PoolAt> {
    let read = body_at(&code.pool, at)?;
    match read.record {
        BodyRecord::Inline { entry } => out.push(WalkPlace::Program { entry, unit }),
        BodyRecord::Call {
            entry, unit: own, ..
        } => out.push(WalkPlace::Program { entry, unit: own }),
        BodyRecord::Table { index, .. } => {
            if site_table(code, index, out) == FillRuns::NotYet {
                return None;
            }
        }
    }
    Some(read.after)
}
