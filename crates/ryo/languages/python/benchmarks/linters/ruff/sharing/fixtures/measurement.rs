//! The measurement of the one path over one workload, the Django canary it runs and the row it records.
use ryo_benchmark::inbound::use_cases::run_ryo::use_case::RunRyoUseCaseImpl;
use ryo_benchmark::outbound::engine::factory;
use ryo_benchmark::inbound::use_cases::run_ryo::interface::RunRyoUseCase;
use ryo_benchmark::outbound::worker::factory as workers;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;

/// The Django canary request the runbook names: the sealed corpus and the cold request that lists its files.
pub(crate) const DJANGO_REQUEST: &str = "/tmp/ryo-event-ready-django213-20261105/cold-request.json";
/// The one rule of the Django canary.
pub(crate) const DJANGO_RULE: &str = "N999";
/// The frontend's rule for a file the parser rejects, reported beside the canary's rule.
pub(crate) const FRONTEND_RULE: &str = "invalid-syntax";
/// The Django canary's diagnostics per rule at the runbook baseline: 110 N999 and 2 invalid-syntax, 112 in all.
pub(crate) const DJANGO_SPLIT: [(&str, u64); 2] = [(DJANGO_RULE, 110), (FRONTEND_RULE, 2)];

/// What the one path did with one workload.
#[derive(Clone, Debug)]
pub(crate) struct Measured {
    pub workload: &'static str,
    /// The worker's report text, or the error text: compared byte for byte with the baseline's.
    pub output: Result<String, String>,
    pub wall_seconds: f64,
    pub cpu_seconds: Option<f64>,
    pub peak_rss_bytes: Option<u64>,
}

#[derive(Clone, Default)]
pub(crate) struct Capture(Arc<Mutex<Vec<u8>>>);
impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("capture poisoned"))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(crate) fn process_cpu_seconds() -> Option<f64> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let after = stat.rsplit_once(')')?.1;
    let fields: Vec<&str> = after.split_whitespace().collect();
    let ticks = fields.get(11)?.parse::<f64>().ok()? + fields.get(12)?.parse::<f64>().ok()?;
    Some(ticks / 100.0)
}

pub(crate) fn peak_rss_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmHWM:"))?;
    let kilobytes = line.split_whitespace().nth(1)?.parse::<u64>().ok()?;
    Some(kilobytes * 1024)
}

/// Resets the peak resident size so the next reading belongs to the next run alone.
pub(crate) fn reset_peak() {
    let _ignored: std::io::Result<()> = std::fs::write("/proc/self/clear_refs", "5");
}

/// Runs the worker payload through the engine, measuring the run.
pub(crate) fn measured(
    workload: &'static str,
    payload: String,
) -> Result<Measured, Box<dyn std::error::Error>> {
    let captured = Capture::default();
    let worker = workers::json(
        ryo_benchmark::outbound::worker::types::WorkerSource::Argument(payload.into()),
        Box::new(captured.clone()),
    );
    let engine = factory::local_with_frontend_diagnostic(
        ryo::linter::Languages(vec![ryo_python::linked_with_settings(
            ryo_python::settings::PythonParserSettings::new(
                ryo_python::settings::PythonTargetVersion::Py310,
            ),
        )]),
        None,
        ryo_benchmark::outbound::engine::types::FrontendDiagnosticProjection {
            rule: ryo::linter::RuleId::try_from(FRONTEND_RULE)?,
            query: ryo::query::RyoQlQuery::try_from(
                crate::frontend_projection::constants::FRONTEND_DIAGNOSTIC_QUERY,
            )?,
        },
    );
    reset_peak();
    let cpu = process_cpu_seconds();
    let started = Instant::now();
    let run = RunRyoUseCaseImpl::new(
        Arc::new(engine),
        Arc::new(worker),
        Arc::new(ryo_benchmark::outbound::context::factory::local()),
    )
    .run_ryo();
    let wall_seconds = started.elapsed().as_secs_f64();
    let cpu_seconds = process_cpu_seconds().zip(cpu).map(|(after, before)| after - before);
    let text = String::from_utf8(captured.0.lock().map_err(|_| "capture poisoned")?.clone())?;
    Ok(Measured {
        workload,
        output: run.map(|()| text).map_err(|error| error.to_string()),
        wall_seconds,
        cpu_seconds,
        peak_rss_bytes: peak_rss_bytes(),
    })
}

/// The Django canary: the runbook's cold request narrowed to its one rule, with a fresh cache.
pub(crate) fn django() -> Result<Measured, Box<dyn std::error::Error>> {
    let mut request: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(DJANGO_REQUEST)?)?;
    let cache = tempfile::tempdir()?;
    request["selected_rules"] = serde_json::json!([DJANGO_RULE]);
    request["cache"] = serde_json::json!(cache.path().join("cache"));
    // The request predates the retired strategy selectors: their two removed keys are dropped, nothing else is changed.
    if let Some(performance) = request["performance"].as_object_mut() {
        let _column = performance.remove("column_kernel");
        let _reuse = performance.remove("node_result_reuse");
    }
    measured("django-n999", serde_json::to_string(&request)?)
}

pub(crate) fn digest(output: &Result<String, String>) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;
    let mut hash = Sha256::new();
    match output {
        Ok(text) => hash.update(text.as_bytes()),
        Err(error) => hash.update(format!("error: {error}").as_bytes()),
    }
    hash.finalize().iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    })
}

/// The diagnostics the directional checkpoint recorded for the Django canary, read from its measurement.
pub(crate) fn recorded_django_diagnostics() -> Result<u64, Box<dyn std::error::Error>> {
    let measured: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/contracts/evidence/checkpoint/measurements.json"),
    )?)?;
    measured["diagnostics"]
        .as_u64()
        .ok_or_else(|| "the checkpoint measurement records no diagnostics".into())
}

/// How many diagnostics a worker report holds: every entry of its `diagnostics` list, the rule's and the frontend's
/// `invalid-syntax` alike, which is what the checkpoint measurement counted (110 N999 and 2 invalid-syntax over Django).
pub(crate) fn diagnostics(report: &str) -> Result<u64, Box<dyn std::error::Error>> {
    Ok(u64::try_from(entries(report)?.len())?)
}

/// How many diagnostics of one rule a worker report holds.
pub(crate) fn diagnostics_of(report: &str, rule: &str) -> Result<u64, Box<dyn std::error::Error>> {
    Ok(u64::try_from(entries(report)?.iter().filter(|entry| entry["rule"] == rule).count())?)
}

/// The entries of a worker report's `diagnostics` list.
fn entries(report: &str) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let mut report: serde_json::Value = serde_json::from_str(report)?;
    match report["diagnostics"].take() {
        serde_json::Value::Array(entries) => Ok(entries),
        _ => Err("the worker report holds no diagnostics list".into()),
    }
}

/// Writes the one path's row into the results directory beside its README.
pub(crate) fn record(run: &Measured) -> Result<(), Box<dyn std::error::Error>> {
    let row = serde_json::json!({
        "workload": run.workload,
        "failed": run.output.is_err(),
        "output_sha256": digest(&run.output),
        "wall_seconds": run.wall_seconds,
        "cpu_seconds": run.cpu_seconds,
        "peak_rss_bytes": run.peak_rss_bytes,
    });
    let target = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("benchmarks/linters/ruff/results/route-differential-2026-11-07/results.json");
    fs::write(target, serde_json::to_string_pretty(&[row])?)?;
    Ok(())
}
