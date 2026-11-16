//! The one document of a bench run: every measure added under its benchmark, and the names they are written with.
use super::constants;
use super::schemas::MetricSchema;
use super::types::{BenchmarkName, MeasureDocument, MeasureName, Observed};
use crate::outbound::evidence::types::{RuleTiming, RuleTimings, VerificationTimesFiles};
use ryo_benchmark_contracts::domain::measurements::types::ShapeGrowths;
use ryo_benchmark_contracts::domain::measurements::types::{ProjectCosts, RunCost};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage;
use std::collections::BTreeMap;
use std::time::Duration;

/// A measure observed once: its value is its lower and upper value.
fn once(value: f64) -> Observed {
    Observed {
        value,
        lower: value,
        upper: value,
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
        value,
        lower,
        upper,
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
            ryo_benchmark_contracts::domain::reports::constants::NO_MEASURED_EVIDENCE.into(),
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
            ryo_benchmark_contracts::domain::reports::constants::NO_MEASURED_EVIDENCE.into(),
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
                                value: observed.value,
                                lower_value: observed.lower,
                                upper_value: observed.upper,
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
#[cfg(test)]
#[path = "document_test.rs"]
mod tests;
