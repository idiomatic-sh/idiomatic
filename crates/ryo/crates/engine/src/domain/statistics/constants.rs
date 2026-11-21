//! The constants of the `statistics` concept: the three lines statistics start with, as bytes.

use super::types::StatisticsBytes;

/// The first line of statistics: what says the bytes are statistics.
pub(crate) const HEADING: StatisticsBytes<'static> = StatisticsBytes(b"ryo-statistics");

/// The third line of the statistics of a first sample.
pub(crate) const FIRST: StatisticsBytes<'static> = StatisticsBytes(b"first");

/// The third line of the statistics of a second sample.
pub(crate) const SECOND: StatisticsBytes<'static> = StatisticsBytes(b"second");

/// What ends each line of the header.
pub(crate) const LINE_END: StatisticsBytes<'static> = StatisticsBytes(b"\n");
