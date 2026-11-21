//! The constants of the `statistics` concept: the three lines statistics start with, as bytes; the numbers a unit's name,
//! form, shape and order are written as; and the bound a measured order is held to.

use super::types::StatisticsBytes;
use ryo_relations::domain::relations::types::Count;

/// The first line of statistics: what says the bytes are statistics.
pub(crate) const HEADING: StatisticsBytes<'static> = StatisticsBytes(b"ryo-statistics");

/// The third line of the statistics of a first sample.
pub(crate) const FIRST: StatisticsBytes<'static> = StatisticsBytes(b"first");

/// The third line of the statistics of a second sample.
pub(crate) const SECOND: StatisticsBytes<'static> = StatisticsBytes(b"second");

/// What ends each line of the header.
pub(crate) const LINE_END: StatisticsBytes<'static> = StatisticsBytes(b"\n");

/// The width of a number: every count, column and code after the header is eight bytes, the low byte first.
pub(crate) const NUMBER_WIDTH: Count = Count(size_of::<u64>());

/// What a unit named as a definition's pattern is written as.
pub(crate) const PATTERN_UNIT: Count = Count(0);

/// What a unit named as a rule's own program is written as.
pub(crate) const RULE_UNIT: Count = Count(1);

/// What a unit named as a table's fill is written as.
pub(crate) const TABLE_UNIT: Count = Count(2);

/// What the first of two choices is written as: the call form, a decided shape, a learned order.
pub(crate) const FIRST_CHOICE: Count = Count(0);

/// What the second of two choices is written as: the table form, an open shape, a denied order.
pub(crate) const SECOND_CHOICE: Count = Count(1);

/// The share of a unit's first cost its second run may pass before its measured order is denied: one part in this many.
pub(crate) const ORDER_SHARE: Count = Count(50);

/// The steps a unit's second run may cost more than that share before its measured order is denied.
pub(crate) const ORDER_STEPS: Count = Count(50);
