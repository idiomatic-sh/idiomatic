//! The services of the `statistics` concept: statistics read from their bytes (`bytes::service`), learned from a sample or
//! from two (`learning::service`), and asked for a unit. Statistics hold counted work and no time; what a sample sized (the
//! ids of each kind, the rows of each constant bucket) is not written: the lowering costs by its fixed priors and reads no
//! size, so `sizes` is empty in every statistics read.

use super::bytes::service::{read, written};
use super::learning::service::{decided, sampled, unit_named};
use super::types::{
    EncodedStatistics, LearnStage, MatchedUnits, Measured, MeasuredSizes, MeasuredUnit, Statistics,
    StatisticsBytes,
};
use crate::domain::code::types::{UnitCode, UnitKey};
use ryo_values::domain::descriptions::types::KindVocabulary;

/// The statistics the bytes hold, or none when they are of another engine version or do not read: absent, never an error.
/// Statistics that measured no unit are their header alone; a unit of another rule set, or of a rule since edited, is read
/// like any other and names no unit of the programs being loaded, so it changes nothing.
#[must_use]
pub(crate) fn decoded(bytes: StatisticsBytes<'_>, names: &KindVocabulary) -> Option<Statistics> {
    // The kinds a sample sized would be named by the vocabulary's words: no size is written, so no name is read.
    let _ = names;
    read(bytes)
}

/// Statistics of a stage and its units, as bytes.
fn staged(stage: LearnStage, units: Vec<MeasuredUnit>) -> EncodedStatistics {
    written(&Statistics {
        stage,
        sizes: MeasuredSizes::default(),
        units,
    })
}

/// The statistics of a sample run: each unit's counts in the form it was run in, named by its node, so a unit of another
/// rule set matches nothing. With the second run of the same sample (the one loaded from the first's statistics), each flip
/// candidate keeps the form that did less counted work and each unit its order verdict; the counts written are the first's.
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
    let units = sampled(first);
    match second {
        None => staged(LearnStage::First, units),
        Some(second) => staged(LearnStage::Second, decided(&units, &sampled(second))),
    }
}

/// The statistics a sample teaches a rule set that was loaded with `given`: what `learned` answers, for a host that runs
/// each sample in a process of its own and holds the first's statistics as bytes. Loaded with none (or none that read),
/// the sample is the first: `learned(sample, None)`. Loaded with a first sample's, it is the second, and the answer is
/// `learned(first, Some(sample))`. Loaded with a second's, nothing more is learned: they are answered as given.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_engine::domain::statistics::service::{learned, learned_from};
/// use ryo_engine::domain::statistics::types::{Measured, PointCounts, StatisticsBytes};
///
/// let (programs, counts) = (Programs::default(), PointCounts::default());
/// let sample = Measured { programs: &programs, counts: &counts };
/// assert_eq!(learned_from(StatisticsBytes(&[]), sample), learned(sample, None));
/// let first = learned(sample, None);
/// assert_eq!(learned_from(StatisticsBytes(&first.0), sample), learned(sample, Some(sample)));
/// ```
#[must_use]
pub fn learned_from(given: StatisticsBytes<'_>, sample: Measured<'_>) -> EncodedStatistics {
    match read(given) {
        None => learned(sample, None),
        Some(first) => match first.stage {
            LearnStage::First => {
                staged(LearnStage::Second, decided(&first.units, &sampled(sample)))
            }
            LearnStage::Second => staged(LearnStage::Second, first.units),
        },
    }
}

/// The unit of the statistics a name names, when they measured one: the one lookup of a unit by its name.
#[must_use]
pub(crate) fn unit_of<'s>(statistics: &'s Statistics, key: &UnitKey) -> Option<&'s MeasuredUnit> {
    unit_named(&statistics.units, key)
}

/// How many of `units` the statistics name: a unit of the programs is matched by its key.
pub(crate) fn matched(statistics: &Statistics, units: &[UnitCode]) -> MatchedUnits {
    let named = units
        .iter()
        .filter(|unit| unit_of(statistics, &unit.key).is_some());
    MatchedUnits(named.count())
}
