//! Only this boundary knows the BMF field and metric names.
use super::constants;
use crate::outbound::evidence::types::measured_publication::value::MeasuredPublication;
use ryo_benchmark_contracts::domain::measurements::types::PublicationSample;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{
    Comparison, ComparisonContract, ExecutionMetadata, ReportComparison, RunReport,
};
use ryo_benchmark_contracts::domain::reports::types::{
    EndPair, FindingPosition, LocatedFinding, ParityReport, RuleComparison,
};
use serde::Serialize;
use super::types::{ParityCount, ParityCounts, ReportName};
use std::collections::BTreeMap;

/// Median and observed extrema for one BMF metric.
#[derive(Serialize)]
struct MetricSchema {
    /// Median across the complete sample group.
    value: f64,
    /// Smallest observed sample.
    lower_value: f64,
    /// Largest observed sample.
    upper_value: f64,
}

/// Wire-level measurements extracted without validation or loss from one sample.
struct ObservationSchema<'a> {
    /// Stable project, contestant and condition identity.
    name: &'a str,
    /// Target process elapsed milliseconds.
    wall: f64,
    /// Target process CPU milliseconds, absent without collection.
    cpu: Option<f64>,
    /// Target process peak resident bytes, absent without collection.
    resident: Option<u64>,
}
impl<'a> From<&'a PublicationSample<'_>> for ObservationSchema<'a> {
    fn from(sample: &'a PublicationSample<'_>) -> Self {
        Self {
            name: &sample.name.0,
            wall: sample.measurement.wall.as_secs_f64() * 1000.0,
            cpu: match sample.measurement.resources {
                ryo_benchmark_contracts::domain::measurements::types::ResourceObservation::WallOnly => None,
                ryo_benchmark_contracts::domain::measurements::types::ResourceObservation::Collected(resources) => {
                    Some(resources.cpu.as_secs_f64() * 1000.0)
                }
            },
            resident: match sample.measurement.resources {
                ryo_benchmark_contracts::domain::measurements::types::ResourceObservation::WallOnly => None,
                ryo_benchmark_contracts::domain::measurements::types::ResourceObservation::Collected(resources) => {
                    Some(resources.peak_rss_bytes.0)
                }
            },
        }
    }
}
/// Serialize exact resource samples into grouped median and extrema metrics.
pub(super) fn encode(publication: &MeasuredPublication<'_>) -> Result<Vec<u8>, Error> {
    let samples = publication.samples();
    if samples.is_empty() {
        return Err(Error::Invalid(
            ryo_benchmark_contracts::domain::reports::constants::NO_MEASURED_EVIDENCE.into(),
        ));
    }
    if samples.iter().any(|sample| {
        sample.inventory
            != ryo_benchmark_contracts::domain::measurements::types::InventoryStatus::Complete
    }) {
        return Err(Error::Invalid(
            ryo_benchmark_contracts::domain::reports::constants::DIAGNOSTIC_OBSERVATIONS_CANNOT_PUBLISH.into(),
        ));
    }
    let mut grouped: BTreeMap<&str, BTreeMap<&str, Vec<f64>>> = BTreeMap::new();
    for sample in samples.iter().map(ObservationSchema::from) {
        if sample.name.is_empty() {
            return Err(Error::Invalid("empty measurement identity".into()));
        }
        let (Some(cpu), Some(resident)) = (sample.cpu, sample.resident) else {
            return Err(Error::Invalid(super::constants::MISSING_RESOURCES.into()));
        };
        if resident > super::constants::MAX_EXACT_RESIDENT {
            return Err(Error::Invalid(super::constants::INEXACT_RESIDENT.into()));
        }
        let resident = serde_json::Number::from(resident)
            .as_f64()
            .ok_or_else(|| Error::Invalid(super::constants::INEXACT_RESIDENT.into()))?;
        let metrics = grouped.entry(sample.name).or_default();
        for (metric, value) in [
            ("wall-ms", sample.wall),
            ("cpu-ms", cpu),
            ("peak-rss-bytes", resident),
        ] {
            metrics.entry(metric).or_default().push(value);
        }
    }
    let mut document = BTreeMap::new();
    for (name, metrics) in grouped {
        let mut summaries = BTreeMap::new();
        for (metric, mut values) in metrics {
            values.sort_by(f64::total_cmp);
            let (lower_value, upper_value) = values
                .first()
                .zip(values.last())
                .ok_or_else(|| Error::Invalid(super::constants::EMPTY_METRIC.into()))?;
            let middle = values.len() / 2;
            let (lower, upper) = values.split_at(middle);
            let upper_middle = upper
                .first()
                .ok_or_else(|| Error::Invalid(super::constants::EMPTY_METRIC.into()))?;
            let median = if values.len().is_multiple_of(2_usize) {
                let lower_middle = lower
                    .last()
                    .ok_or_else(|| Error::Invalid(super::constants::EMPTY_METRIC.into()))?;
                (lower_middle + upper_middle) / 2.0_f64
            } else {
                *upper_middle
            };
            let _previous = summaries.insert(
                metric.to_owned(),
                MetricSchema {
                    value: median,
                    lower_value: *lower_value,
                    upper_value: *upper_value,
                },
            );
        }
        let _previous = document.insert(name.to_owned(), summaries);
    }
    serde_json::to_vec(&document).map_err(|error| Error::Json(Box::new(error)))
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

/// One producer's execution metadata as the comparison evidence spells it.
#[derive(Serialize)]
#[serde(tag = "availability", rename_all = "kebab-case")]
enum ExecutionMetadataSchema<'a> {
    Unknown {},
    Unavailable { reason: &'a str },
    Complete {},
}
impl<'a> From<&'a ExecutionMetadata> for ExecutionMetadataSchema<'a> {
    fn from(value: &'a ExecutionMetadata) -> Self {
        match value {
            ExecutionMetadata::Unknown => Self::Unknown {},
            ExecutionMetadata::Unavailable { reason } => Self::Unavailable { reason: &reason.0 },
            ExecutionMetadata::Complete => Self::Complete {},
        }
    }
}

/// The name of one comparison outcome.
const fn comparison_name(value: Comparison) -> &'static str {
    match value {
        Comparison::Exact => constants::EXACT,
        Comparison::Different => constants::DIFFERENT,
        Comparison::Incomparable => constants::INCOMPARABLE,
    }
}

/// Both comparison dimensions as the comparison evidence spells them.
#[derive(Serialize)]
struct DimensionsSchema {
    diagnostics: &'static str,
    execution_metadata: &'static str,
}
impl From<ReportComparison> for DimensionsSchema {
    fn from(comparison: ReportComparison) -> Self {
        Self {
            diagnostics: comparison_name(comparison.diagnostics),
            execution_metadata: comparison_name(comparison.execution_metadata),
        }
    }
}

/// The comparison evidence line: the declared contract, both dimensions and both producers' capabilities.
#[derive(Serialize)]
struct ComparisonSchema<'a> {
    schema: &'static str,
    contract: &'static str,
    comparison: DimensionsSchema,
    reference_execution_metadata: ExecutionMetadataSchema<'a>,
    candidate_execution_metadata: ExecutionMetadataSchema<'a>,
}

/// The comparison evidence line as one JSON text.
pub(super) fn comparison(
    reference: &RunReport,
    candidate: &RunReport,
    comparison: ReportComparison,
    contract: ComparisonContract,
) -> Result<String, Error> {
    serde_json::to_string(&ComparisonSchema {
        schema: constants::BENCHMARK_COMPARISON_V1,
        contract: match contract {
            ComparisonContract::Diagnostics => constants::DIAGNOSTICS,
            ComparisonContract::CompleteReport => constants::COMPLETE_REPORT,
        },
        comparison: DimensionsSchema::from(comparison),
        reference_execution_metadata: ExecutionMetadataSchema::from(&reference.execution_metadata),
        candidate_execution_metadata: ExecutionMetadataSchema::from(&candidate.execution_metadata),
    })
    .map_err(|error| Error::Json(Box::new(error)))
}

/// One count metric of a parity report.
#[derive(Serialize)]
struct CountSchema {
    /// The count.
    value: f64,
}

/// A count as an exact BMF number.
fn count(value: usize) -> Result<CountSchema, Error> {
    u32::try_from(value)
        .map(|value| CountSchema {
            value: f64::from(value),
        })
        .map_err(|_too_many| Error::Invalid(constants::EMPTY_METRIC.into()))
}

/// The counts of one parity report under the reference's name: its differing rules and unaccepted differences.
pub(super) fn parity_counts(parity: &ParityReport<'_>) -> (ReportName, ParityCounts) {
    let differing = parity
        .rules
        .iter()
        .filter(|rule| !rule.missing.is_empty() || !rule.extra.is_empty())
        .count();
    (
        ReportName(parity.reference.0.to_owned()),
        ParityCounts {
            differing: ParityCount(differing),
            unaccepted: ParityCount(parity.problems.len()),
        },
    )
}

/// Every parity report's counts as one BMF document, each under its reference's name.
pub(super) fn parity_document(
    reports: &BTreeMap<ReportName, ParityCounts>,
) -> Result<Vec<u8>, Error> {
    let mut document = BTreeMap::new();
    for (name, counts) in reports {
        let _previous = document.insert(
            name.0.as_str(),
            BTreeMap::from([
                (constants::DIFFERING_RULES, count(counts.differing.0)?),
                (constants::UNACCEPTED_DIFFERENCES, count(counts.unaccepted.0)?),
            ]),
        );
    }
    serde_json::to_vec(&document).map_err(|error| Error::Json(Box::new(error)))
}

/// A position as the per-rule document spells an end: `[line, column]`.
const fn end(position: FindingPosition) -> [usize; 2] {
    [position.line.0, position.column.0]
}

/// A located finding as a sample: file, line, column and end.
struct FindingSampleSchema<'a>(&'a LocatedFinding);

impl Serialize for FindingSampleSchema<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap as _;
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("file", self.0.file.0.as_ref())?;
        map.serialize_entry("line", &self.0.start.line.0)?;
        map.serialize_entry("column", &self.0.start.column.0)?;
        map.serialize_entry("end", &end(self.0.end))?;
        map.end()
    }
}

/// A pair of findings whose ends differ, as a sample: file, line, column and each contestant's end under its name.
struct EndSampleSchema<'a> {
    /// The candidate's end key.
    candidate: &'a EndKeySchema,
    /// The reference's end key.
    reference: &'a EndKeySchema,
    /// The paired findings.
    pair: &'a EndPair<LocatedFinding>,
}

/// A contestant's end key in an end-difference sample: its name followed by `_end`.
struct EndKeySchema(String);

impl Serialize for EndSampleSchema<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap as _;
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("file", self.pair.candidate.file.0.as_ref())?;
        map.serialize_entry("line", &self.pair.candidate.start.line.0)?;
        map.serialize_entry("column", &self.pair.candidate.start.column.0)?;
        map.serialize_entry(&self.candidate.0, &end(self.pair.candidate.end))?;
        map.serialize_entry(&self.reference.0, &end(self.pair.reference.end))?;
        map.end()
    }
}

/// The first located findings of a kind, as samples.
fn finding_samples(located: &[LocatedFinding]) -> Vec<FindingSampleSchema<'_>> {
    located
        .iter()
        .take(constants::SAMPLES)
        .map(FindingSampleSchema)
        .collect()
}

/// One per-rule row, its keys in document order and the contestants' counts under their names.
struct RuleRowSchema<'a> {
    /// The report the row belongs to.
    parity: &'a ParityReport<'a>,
    /// The rule's comparison.
    rule: &'a RuleComparison<LocatedFinding>,
}

impl Serialize for RuleRowSchema<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap as _;
        let rule = self.rule;
        let candidate = self.parity.candidate.0;
        let reference = self.parity.reference.0;
        let candidate_end = EndKeySchema(format!("{candidate}{}", constants::END_KEY));
        let reference_end = EndKeySchema(format!("{reference}{}", constants::END_KEY));
        let class = match (rule.missing.is_empty(), rule.extra.is_empty()) {
            (true, true) => constants::IDENTICAL.to_owned(),
            (false, false) => constants::BOTH.to_owned(),
            (false, true) => format!("{candidate}{}", constants::MISSING_CLASS),
            (true, false) => format!("{candidate}{}", constants::EXTRA_CLASS),
        };
        let mut map = serializer.serialize_map(Some(12))?;
        map.serialize_entry("rule", rule.rule.as_ref())?;
        map.serialize_entry(candidate, &(rule.matched.0 + rule.extra.len()))?;
        map.serialize_entry(reference, &(rule.matched.0 + rule.missing.len()))?;
        map.serialize_entry("matched", &rule.matched.0)?;
        map.serialize_entry("missing", &rule.missing.len())?;
        map.serialize_entry("extra", &rule.extra.len())?;
        map.serialize_entry("end_differs", &rule.end_differences.len())?;
        map.serialize_entry("missing_samples", &finding_samples(&rule.missing))?;
        map.serialize_entry("extra_samples", &finding_samples(&rule.extra))?;
        map.serialize_entry(
            "end_differs_samples",
            &rule
                .end_differences
                .iter()
                .take(constants::SAMPLES)
                .map(|pair| EndSampleSchema {
                    candidate: &candidate_end,
                    reference: &reference_end,
                    pair,
                })
                .collect::<Vec<_>>(),
        )?;
        map.serialize_entry("class", &class)?;
        map.serialize_entry("differences", &(rule.missing.len() + rule.extra.len()))?;
        map.end()
    }
}

/// The per-rule document: under `identical`, on one line, the finding count of each rule whose findings and ends agree; under `rules`,
/// every other selected rule's row; both in selection order, indented by one space.
pub(super) fn parity_rules(parity: &ParityReport<'_>) -> Result<Vec<u8>, Error> {
    let json = |error| Error::Json(Box::new(error));
    let (agree, differ): (Vec<_>, Vec<_>) = parity.rules.iter().partition(|rule| {
        rule.missing.is_empty() && rule.extra.is_empty() && rule.end_differences.is_empty()
    });
    let mut counts = Vec::new();
    for rule in agree {
        counts.push(format!(
            "{}:{}",
            serde_json::to_string(rule.rule.as_ref()).map_err(json)?,
            rule.matched.0
        ));
    }
    let rows: Vec<_> = differ
        .into_iter()
        .map(|rule| RuleRowSchema { parity, rule })
        .collect();
    let mut pretty = Vec::new();
    let indent = constants::PER_RULE_INDENT;
    let mut serializer = serde_json::Serializer::with_formatter(
        &mut pretty,
        serde_json::ser::PrettyFormatter::with_indent(indent),
    );
    rows.serialize(&mut serializer).map_err(json)?;
    let rows = String::from_utf8_lossy(&pretty).replace('\n', "\n ");
    Ok(format!(
        "{{\n \"identical\": {{{}}},\n \"rules\": {rows}\n}}",
        counts.join(",")
    )
    .into_bytes())
}
