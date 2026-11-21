//! The services of the `statistics` concept. Statistics are bytes: a header of three lines (the word that says they are
//! statistics, the version of the engine that wrote them, the stage of the sample that wrote them), then what the sample
//! measured. A file of a first sample that measured no unit is its header alone.

use super::constants::{FIRST, HEADING, LINE_END, SECOND};
use super::types::{
    EncodedStatistics, LearnStage, MatchedUnits, Measured, MeasuredSizes, Statistics,
    StatisticsBytes,
};
use crate::domain::code::types::{UnitCode, UnitKey};
use crate::domain::packs::constants::ENGINE_VERSION;
use ryo_values::domain::descriptions::types::KindVocabulary;
use std::collections::BTreeSet;

/// The header of statistics of `stage` written by this engine: its three lines, each ended.
fn header(stage: LearnStage) -> EncodedStatistics {
    let stage = match stage {
        LearnStage::First => FIRST,
        LearnStage::Second => SECOND,
    };
    let mut bytes = Vec::new();
    for line in [HEADING.0, ENGINE_VERSION.as_bytes(), stage.0] {
        bytes.extend_from_slice(line);
        bytes.extend_from_slice(LINE_END.0);
    }
    EncodedStatistics(bytes)
}

/// The statistics the bytes hold, or none when they are of another engine version or do not read: absent, never an error.
/// What is read today is the header: statistics that measured no unit, of either stage. Bytes after the header are the
/// units and the sizes of a sample, which nothing writes yet and nothing reads: they do not read, and are absent.
#[must_use]
pub(crate) fn decoded(bytes: StatisticsBytes<'_>, names: &KindVocabulary) -> Option<Statistics> {
    // The kinds a sample sized are named by the vocabulary's words: a header sizes none.
    let _ = names;
    let stage = [LearnStage::First, LearnStage::Second]
        .into_iter()
        .find(|stage| header(*stage).0 == bytes.0)?;
    Some(Statistics {
        stage,
        sizes: MeasuredSizes::default(),
        units: Vec::new(),
    })
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
