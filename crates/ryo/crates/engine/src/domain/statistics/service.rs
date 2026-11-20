//! The services of the `statistics` concept.

use super::types::{EncodedStatistics, MatchedUnits, Measured, Statistics, StatisticsBytes};
use crate::domain::code::types::{UnitCode, UnitKey};
use ryo_values::domain::descriptions::types::KindVocabulary;
use std::collections::BTreeSet;

/// The statistics the bytes hold, or none when they are of another engine version or do not read: absent, never an error.
#[must_use]
pub(crate) fn decoded(bytes: StatisticsBytes<'_>, names: &KindVocabulary) -> Option<Statistics> {
    // The bytes are the passes' track's: until it reads them every load runs with none.
    let _ = (bytes, names);
    None
}

/// The statistics of a sample run: its sizes, and each unit's counts in the form it was run in. With the second run of the same
/// sample (the one loaded from the first's statistics), each flip candidate keeps the form that cost less and each unit its
/// order verdict.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_engine::domain::statistics::service::learned;
/// use ryo_engine::domain::statistics::types::{Measured, PointCounts};
///
/// let (programs, counts) = (Programs::default(), PointCounts::default());
/// let _statistics = learned(Measured { programs: &programs, counts: &counts }, None);
/// ```
#[must_use]
pub fn learned(first: Measured<'_>, second: Option<Measured<'_>>) -> EncodedStatistics {
    // The bytes are the passes' track's: until it writes them a sample teaches nothing, and a load reads none.
    let _ = (first, second);
    EncodedStatistics::default()
}

/// How many of `units` the statistics name: a unit of the programs is matched by its key.
pub(crate) fn matched(statistics: &Statistics, units: &[UnitCode]) -> MatchedUnits {
    let measured: BTreeSet<&UnitKey> = statistics.units.iter().map(|unit| &unit.key).collect();
    let named = units.iter().filter(|unit| measured.contains(&unit.key));
    MatchedUnits(named.count())
}
