//! The one document of a bench run: every measure added under its benchmark, and the names they are written with; and
//! the comparison evidence line, as JSON on the comparison stream.
use super::constants;
use super::types::{BenchmarkName, MeasureDocument, MeasureName, MeasureValue, Observed};
use crate::outbound::evidence::types::{
    MeasureClass, PublishedDocument, RuleTiming, RuleTimings, VerificationTimesFiles,
};
use benchmark_contracts::domain::costs::types::{ProjectCosts, RunCost};
use benchmark_contracts::domain::measurements::types::ShapeGrowths;
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use benchmark_contracts::domain::reports::types::{
    Comparison, ComparisonContract, ExecutionMetadata, ReportComparison, RunReport,
};
use benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage;
use serde::Serialize;
use std::collections::BTreeMap;
use std::time::Duration;

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

/// One measure as BMF writes it: its value, and the lowest and highest it was.
#[derive(Serialize)]
struct MetricSchema {
    /// The median of the trials, or the one observation.
    value: f64,
    /// The lowest trial.
    lower_value: f64,
    /// The highest trial.
    upper_value: f64,
}

/// A measure observed once: its value is its lower and upper value.
fn once(value: f64) -> Observed {
    Observed {
        value: MeasureValue(value),
        lower: MeasureValue(value),
        upper: MeasureValue(value),
    }
}

/// A measure observed in several trials: the median is its value, the fastest its lower and the slowest its upper value.
fn trials(values: &[f64]) -> Result<Observed, Error> {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let (Some(lower), Some(upper)) = (sorted.first().copied(), sorted.last().copied()) else {
        return Err(Error::Invalid(constants::EMPTY_METRIC.into()));
    };
    let middle = sorted.len() / 2;
    let value = if sorted.len() % 2 == 1 {
        sorted.get(middle).copied().unwrap_or(lower)
    } else {
        let below = sorted
            .get(middle.saturating_sub(1))
            .copied()
            .unwrap_or(lower);
        let above = sorted.get(middle).copied().unwrap_or(upper);
        f64::midpoint(below, above)
    };
    Ok(Observed {
        value: MeasureValue(value),
        lower: MeasureValue(lower),
        upper: MeasureValue(upper),
    })
}

/// Seconds as a measure's number.
fn seconds(durations: &[Duration]) -> Vec<f64> {
    durations.iter().map(Duration::as_secs_f64).collect()
}

/// Bytes, or a count, as a measure's number: a value above 2^53 is refused, as BMF numbers are floating.
fn bytes(values: impl IntoIterator<Item = u64>) -> Result<Vec<f64>, Error> {
    values
        .into_iter()
        .map(|value| {
            (value <= constants::MAX_EXACT_RESIDENT)
                .then(|| serde_json::Number::from(value).as_f64())
                .flatten()
                .ok_or_else(|| Error::Invalid(constants::INEXACT_RESIDENT.into()))
        })
        .collect()
}

/// Add one benchmark to `document`, refusing one already there.
fn add(
    document: &mut MeasureDocument,
    benchmark: String,
    measures: BTreeMap<MeasureName, Observed>,
) -> Result<(), Error> {
    if document.0.contains_key(&BenchmarkName(benchmark.clone())) {
        return Err(Error::Invalid(format!(
            "{}: {benchmark}",
            constants::DUPLICATE_BENCHMARK
        )));
    }
    let _previous = document.0.insert(BenchmarkName(benchmark), measures);
    Ok(())
}

/// The CPU, and the peak, of one condition's trials.
fn condition(
    run: &RunCost,
    cpu: &'static str,
    peak: &'static str,
) -> Result<[(MeasureName, Observed); 2], Error> {
    Ok([
        (MeasureName(cpu), trials(&seconds(&run.cpu))?),
        (
            MeasureName(peak),
            trials(&bytes(run.peak.iter().map(|peak| peak.0))?)?,
        ),
    ])
}

/// Add each project's candidate cost as `<language>/<project>`: the cold run's CPU, wall and peak, the warm and edited runs'
/// CPU and peak, and the store the cold run left.
pub(super) fn costs(
    document: &mut MeasureDocument,
    language: &SuiteLanguage,
    costs: &ProjectCosts,
) -> Result<(), Error> {
    if costs.0.is_empty() {
        return Err(Error::Invalid(
            benchmark_contracts::domain::reports::constants::NO_MEASURED_EVIDENCE.into(),
        ));
    }
    for cost in &costs.0 {
        let mut measures = BTreeMap::new();
        measures.extend(condition(
            &cost.cold,
            constants::CPU_S,
            constants::PEAK_RSS_BYTES,
        )?);
        let _wall = measures.insert(
            MeasureName(constants::WALL_S),
            trials(&seconds(&cost.cold.wall))?,
        );
        measures.extend(condition(
            &cost.warm,
            constants::WARM_CPU_S,
            constants::WARM_PEAK_RSS_BYTES,
        )?);
        measures.extend(condition(
            &cost.edited,
            constants::EDITED_CPU_S,
            constants::EDITED_PEAK_RSS_BYTES,
        )?);
        let [store_bytes, store_files] = bytes([cost.store.bytes.0, cost.store.files.0])?
            .try_into()
            .map_err(|_unexpected: Vec<f64>| Error::Invalid(constants::EMPTY_METRIC.into()))?;
        let _bytes = measures.insert(MeasureName(constants::STORE_BYTES), once(store_bytes));
        let _files = measures.insert(MeasureName(constants::STORE_FILES), once(store_files));
        add(
            document,
            format!("{}/{}", language.0, cost.project.0),
            measures,
        )?;
    }
    Ok(())
}

/// Milliseconds as a measure's number.
fn milliseconds(elapsed: Duration) -> f64 {
    elapsed.as_secs_f64() * 1000.0
}

/// Add each rule's verification times as `<language>/verify/<rule>`.
pub(super) fn verification(
    document: &mut MeasureDocument,
    language: &SuiteLanguage,
    timings: &RuleTimings,
) -> Result<(), Error> {
    for (rule, timing) in &timings.0 {
        add(
            document,
            format!("{}/{}/{rule}", language.0, constants::VERIFY),
            BTreeMap::from([
                (
                    MeasureName(constants::TOTAL_MS),
                    once(milliseconds(timing.total)),
                ),
                (
                    MeasureName(constants::MAX_CASE_MS),
                    once(milliseconds(timing.slowest)),
                ),
                (
                    MeasureName(constants::COMPILE_MS),
                    once(milliseconds(timing.compile)),
                ),
            ]),
        )?;
    }
    Ok(())
}

/// Add each shape's growth as `<language>/growth/<shape>`.
pub(super) fn growth(
    document: &mut MeasureDocument,
    language: &SuiteLanguage,
    growth: &ShapeGrowths,
) -> Result<(), Error> {
    for (shape, grown) in &growth.0 {
        add(
            document,
            format!("{}/{}/{}", language.0, constants::GROWTH, shape.0),
            BTreeMap::from([
                (MeasureName(constants::DOUBLING_RATIO), once(grown.ratio.0)),
                (
                    MeasureName(constants::LARGER_MS),
                    once(milliseconds(grown.larger)),
                ),
            ]),
        )?;
    }
    Ok(())
}

/// The document as one line of BMF.
pub(super) fn encode(document: &MeasureDocument) -> Result<Vec<u8>, Error> {
    if document.0.is_empty() {
        return Err(Error::Invalid(
            benchmark_contracts::domain::reports::constants::NO_MEASURED_EVIDENCE.into(),
        ));
    }
    let wire: BTreeMap<&str, BTreeMap<&str, MetricSchema>> = document
        .0
        .iter()
        .map(|(benchmark, measures)| {
            (
                benchmark.0.as_str(),
                measures
                    .iter()
                    .map(|(measure, observed)| {
                        (
                            measure.0,
                            MetricSchema {
                                value: observed.value.0,
                                lower_value: observed.lower.0,
                                upper_value: observed.upper.0,
                            },
                        )
                    })
                    .collect(),
            )
        })
        .collect();
    serde_json::to_vec(&wire).map_err(|error| Error::Json(Box::new(error)))
}

/// The one value of `measure` in `measures`, as a duration of that many milliseconds.
fn measured(
    measures: &serde_json::Map<String, serde_json::Value>,
    measure: &str,
) -> Option<Duration> {
    let milliseconds = measures.get(measure)?.get(constants::VALUE)?.as_f64()?;
    (milliseconds.is_finite() && milliseconds >= 0.0)
        .then(|| Duration::from_secs_f64(milliseconds / 1000.0))
}

/// The times each file holds of its rule: the benchmark ending in `/verify/<rule>`, whatever language it was written under.
pub(super) fn read_verification(files: &VerificationTimesFiles) -> Result<RuleTimings, Error> {
    let mut timings = BTreeMap::new();
    for (rule, file) in &files.0 {
        let text = std::fs::read_to_string(&file.0)?;
        let document: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&text).map_err(|error| Error::Json(Box::new(error)))?;
        let suffix = format!("/{}/{rule}", constants::VERIFY);
        let timing = document
            .iter()
            .find(|(benchmark, _)| benchmark.ends_with(&suffix))
            .and_then(|(_, measures)| measures.as_object())
            .and_then(|measures| {
                Some(RuleTiming {
                    total: measured(measures, constants::TOTAL_MS)?,
                    slowest: measured(measures, constants::MAX_CASE_MS)?,
                    compile: measured(measures, constants::COMPILE_MS)?,
                })
            })
            .ok_or_else(|| {
                Error::Invalid(format!(
                    "{}: {} names no times of {rule}",
                    constants::MISSING_VERIFICATION,
                    file.0.display()
                ))
            })?;
        let _previous = timings.insert(rule.clone(), timing);
    }
    Ok(RuleTimings(timings))
}

/// The written `document` with the measures of `class` alone, a benchmark left with none of them dropped; refused when
/// it names a measure neither class names, or keeps none.
pub(super) fn select(
    document: &PublishedDocument,
    class: MeasureClass,
) -> Result<PublishedDocument, Error> {
    let written: BTreeMap<String, serde_json::Map<String, serde_json::Value>> =
        serde_json::from_slice(&document.0).map_err(|error| Error::Json(Box::new(error)))?;
    let named = match class {
        MeasureClass::Deterministic => constants::DETERMINISTIC.as_slice(),
        MeasureClass::Timing => constants::TIMING.as_slice(),
    };
    let mut kept = BTreeMap::new();
    for (benchmark, mut measures) in written {
        if let Some(unclassed) = measures.keys().map(String::as_str).find(|measure| {
            !constants::DETERMINISTIC.contains(measure) && !constants::TIMING.contains(measure)
        }) {
            return Err(Error::Invalid(format!(
                "{}: {unclassed}",
                constants::UNCLASSED_MEASURE
            )));
        }
        measures.retain(|measure, _value| named.contains(&measure.as_str()));
        let _previous = (!measures.is_empty()).then(|| kept.insert(benchmark, measures));
    }
    if kept.is_empty() {
        return Err(Error::Invalid(constants::NOTHING_SELECTED.into()));
    }
    let mut line = serde_json::to_vec(&kept).map_err(|error| Error::Json(Box::new(error)))?;
    line.extend_from_slice(constants::NEWLINE);
    Ok(PublishedDocument(line))
}

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
