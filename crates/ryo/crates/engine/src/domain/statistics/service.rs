//! The services of the `statistics` concept: statistics read from their bytes (`bytes::service`), learned from a sample or
//! from two (`learning::service`), and asked for a unit or for a size: the elements of a kind or of a field, the rows of a
//! constant bucket, each as the sample's sum and its files.

use super::bytes::service::{read, written};
use super::learning::service::{coded, decided, sampled, sized, unit_named};
use super::types::{
    EncodedStatistics, LearnStage, MatchedUnits, Measured, MeasuredUnit, SizedRows, Statistics,
    StatisticsBytes,
};
use crate::domain::code::types::{UnitCode, UnitKey};
use ryo_language::domain::node_table::types::FieldCode;
use ryo_relations::domain::relations::types::{Column, Count, KindBits, Relation};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::descriptions::types::KindVocabulary;

/// The statistics the bytes hold, or none when they are of another engine version or do not read: absent, never an error.
/// Statistics that measured no unit are their header alone; a unit of another rule set, or of a rule since edited, is read
/// like any other and names no unit of the programs being loaded, so it changes nothing. What the sample sized is read by
/// the names of `names`: a kind and a field the vocabulary does not name are left out.
#[must_use]
pub(crate) fn decoded(bytes: StatisticsBytes<'_>, names: &KindVocabulary) -> Option<Statistics> {
    let held = read(bytes)?;
    Some(Statistics {
        stage: held.stage,
        sizes: coded(&held.sizes, names),
        units: held.units,
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
    let (units, sizes) = (sampled(first), sized(first));
    match second {
        None => written(LearnStage::First, &units, &sizes),
        Some(second) => written(
            LearnStage::Second,
            &decided(&units, &sampled(second)),
            &sizes,
        ),
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
            LearnStage::First => written(
                LearnStage::Second,
                &decided(&first.units, &sampled(sample)),
                &first.sizes,
            ),
            LearnStage::Second => written(LearnStage::Second, &first.units, &first.sizes),
        },
    }
}

/// What a sample measured a size at, when it scanned a file: the rows summed over its files, and the files.
fn sized_at(statistics: &Statistics, rows: Option<Count>) -> Option<SizedRows> {
    let files = statistics.sizes.files;
    (files.0 > 0).then_some(SizedRows { rows: rows?, files })
}

/// The elements of the sample's files: the mean of a file is `rows` over `files`. None when the statistics sized nothing.
#[must_use]
pub(crate) fn element_rows(statistics: &Statistics) -> Option<SizedRows> {
    sized_at(statistics, Some(statistics.sizes.elements))
}

/// The elements of a kind in the sample: what a bucket of the node table named by that kind holds, summed over the
/// sample's files. None when the statistics sized nothing or do not name the kind; a kind the sample never met is
/// `rows` of nought.
#[must_use]
pub(crate) fn kind_rows(statistics: &Statistics, kind: KindBits) -> Option<SizedRows> {
    let kinds = &statistics.sizes.kinds;
    let found = match kinds.binary_search_by_key(&kind, |size| size.kind) {
        Ok(place) => kinds.get(place),
        Err(_) => None,
    };
    sized_at(statistics, found.map(|size| size.ids))
}

/// The elements of the sample that fill a field of their parent: what a bucket of the node table named by that field
/// holds. None when the statistics sized nothing or do not name the field.
#[must_use]
pub(crate) fn field_rows(statistics: &Statistics, field: FieldCode) -> Option<SizedRows> {
    let fields = &statistics.sizes.fields;
    let found = match fields.binary_search_by_key(&field, |size| size.field) {
        Ok(place) => fields.get(place),
        Err(_) => None,
    };
    sized_at(statistics, found.map(|size| size.rows))
}

/// The rows of a filled relation that hold a value in a column, in the sample: `value` is the literal's text (a text as
/// the plan writes it, a number in decimal). None when the statistics sized nothing or the sample's plans did not test
/// that bucket.
#[must_use]
pub(crate) fn literal_rows(
    statistics: &Statistics,
    relation: Relation,
    column: Column,
    value: NameText<'_>,
) -> Option<SizedRows> {
    let mut literals = statistics.sizes.literals.iter();
    let found = literals
        .find(|size| size.relation == relation && size.column == column && *size.value == *value.0);
    sized_at(statistics, found.map(|size| size.rows))
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
