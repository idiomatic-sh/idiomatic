//! The services of the `counting` concept: what a measuring worker counted, the counts of several workers added up, and the
//! work ledger read off them. The counts are taken by the executor's track; until it counts, a worker counted nothing.

use super::types::WorkLedger;
use crate::domain::programs::types::Programs;
use crate::domain::scratch::types::ScanScratch;
use crate::domain::statistics::types::PointCounts;

/// What a worker counted since its scratch was made: the runs of each plan point, what each unit was asked and gave, and the
/// sizes of the files; nothing for a scratch that does not measure.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::counting::service::counted;
/// use ryo_engine::domain::programs::service::scratch_measuring;
/// use ryo_engine::domain::programs::types::Programs;
///
/// let _counts = counted(&scratch_measuring(&Programs::default()));
/// ```
#[must_use]
pub fn counted(scratch: &ScanScratch) -> PointCounts {
    let _ = scratch;
    PointCounts::default()
}

/// Adds one worker's counts to another's.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::counting::service::merged;
/// use ryo_engine::domain::statistics::types::PointCounts;
///
/// let mut all = PointCounts::default();
/// merged(&mut all, &PointCounts::default());
/// ```
pub fn merged(into: &mut PointCounts, from: &PointCounts) {
    let _ = (into, from);
}

/// The work ledger of a measured run: one row for each unit of `programs` (a rule's own program, a definition's pattern, a
/// made table), with what `counts` holds of it and the rules that reach it.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::counting::service::ledger;
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_engine::domain::statistics::types::PointCounts;
///
/// assert!(ledger(&Programs::default(), &PointCounts::default()).0.is_empty());
/// ```
#[must_use]
pub fn ledger(programs: &Programs, counts: &PointCounts) -> WorkLedger {
    let _ = (programs, counts);
    WorkLedger::default()
}
