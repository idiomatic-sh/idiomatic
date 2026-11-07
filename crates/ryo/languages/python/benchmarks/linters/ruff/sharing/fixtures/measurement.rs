//! The configurations, the measurement of one workload, the staged inventory it runs over and the rows it records.
use ryo::linter::RoutePolicy;
use ryo::linter::RouteWiring;
use ryo_benchmark::inbound::use_cases::run_ryo::interface::RunRyoUseCase;
use ryo_benchmark::inbound::use_cases::run_ryo::use_case::RunRyoUseCaseImpl;
use ryo_benchmark::outbound::engine::factory;
use ryo_benchmark::outbound::linter::factory as linters;
use ryo_benchmark::outbound::linter::interface::LinterPort;
use ryo_benchmark::outbound::suite::factory as suites;
use ryo_benchmark::outbound::suite::interface::SuitePort;
use ryo_benchmark::outbound::suite::types::ArtifactPath;
use ryo_benchmark::outbound::worker::factory as workers;
use ryo_benchmark_contracts::domain::requests::types::DiagnosticOptions;
use ryo_benchmark_contracts::domain::requests::types::PerformanceOptions;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use ryo_benchmark_contracts::domain::workloads::types::StagePurpose;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
/// The Django canary request the runbook names: the sealed corpus and the cold request that lists its files.
pub(crate) const DJANGO_REQUEST: &str = "/tmp/ryo-event-ready-django213-20261105/cold-request.json";
/// The one rule of the Django canary (112 diagnostics at the runbook baseline).
pub(crate) const DJANGO_RULE: &str = "N999";

/// One engine configuration: a name and the strategies it wires.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Configuration {
    pub name: &'static str,
    pub policy: RoutePolicy,
}

/// The baseline, each strategy unwired alone, and every strategy unwired: the generic evaluation of the operators.
pub(crate) fn configurations() -> Vec<Configuration> {
    let all = RoutePolicy::default();
    let off = RouteWiring::Unwired;
    vec![
        Configuration {
            name: "all-wired",
            policy: all,
        },
        Configuration {
            name: "without-ordered-existence",
            policy: RoutePolicy {
                ordered_existence: off,
                ..all
            },
        },
        Configuration {
            name: "without-column-kernel",
            policy: RoutePolicy {
                column_kernel: off,
                ..all
            },
        },
        Configuration {
            name: "without-stored-selection",
            policy: RoutePolicy {
                stored_selection: off,
                ..all
            },
        },
        Configuration {
            name: "without-native-positions",
            policy: RoutePolicy {
                native_positions: off,
                ..all
            },
        },
        Configuration {
            name: "without-tree-join",
            policy: RoutePolicy {
                tree_join: off,
                ..all
            },
        },
        Configuration {
            name: "generic-only",
            policy: RoutePolicy {
                ordered_existence: off,
                column_kernel: off,
                stored_selection: off,
                native_positions: off,
                tree_join: off,
            },
        },
    ]
}

/// What one configuration did with one workload.
#[derive(Clone, Debug)]
pub(crate) struct Measured {
    pub configuration: &'static str,
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

/// Runs the worker payload through the engine of `configuration`, measuring the run.
pub(crate) fn measured(
    configuration: Configuration,
    workload: &'static str,
    payload: String,
) -> Result<Measured, Box<dyn std::error::Error>> {
    let captured = Capture::default();
    let worker = workers::json(payload.into(), Box::new(captured.clone()));
    let engine = factory::local_with_routes(
        ryo::linter::Languages(vec![ryo_python::linked_with_settings(
            ryo_python::settings::PythonParserSettings::new(
                ryo_python::settings::PythonTargetVersion::Py310,
            ),
        )]),
        ryo_benchmark::outbound::engine::types::FrontendDiagnosticProjection {
            rule: ryo::linter::RuleId::try_from("invalid-syntax")?,
            query: ryo::query::RyoQlQuery::try_from(
                crate::frontend_projection::constants::FRONTEND_DIAGNOSTIC_QUERY,
            )?,
        },
        configuration.policy,
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
    let cpu_seconds = process_cpu_seconds()
        .zip(cpu)
        .map(|(after, before)| after - before);
    let text = String::from_utf8(captured.0.lock().map_err(|_| "capture poisoned")?.clone())?;
    Ok(Measured {
        configuration: configuration.name,
        workload,
        output: run.map(|()| text).map_err(|error| error.to_string()),
        wall_seconds,
        cpu_seconds,
        peak_rss_bytes: peak_rss_bytes(),
    })
}

/// The Django canary: the runbook's cold request narrowed to its one rule, with a fresh cache.
pub(crate) fn django(configuration: Configuration) -> Result<Measured, Box<dyn std::error::Error>> {
    let mut request: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(DJANGO_REQUEST)?)?;
    let cache = tempfile::tempdir()?;
    request["selected_rules"] = serde_json::json!([DJANGO_RULE]);
    request["cache"] = serde_json::json!(cache.path().join("cache"));
    // The request predates the retired strategy selector: its one removed key is dropped, nothing else is changed.
    if let Some(performance) = request["performance"].as_object_mut() {
        let _removed = performance.remove("column_kernel");
    }
    measured(
        configuration,
        "django-n999",
        serde_json::to_string(&request)?,
    )
}

/// The staged 413-rule profile and the pinned real-code corpus, prepared once for every configuration.
pub(crate) struct Inventory {
    pub(crate) temporary: tempfile::TempDir,
    pub(crate) staged: PathBuf,
    pub(crate) profile: ryo_benchmark_contracts::domain::workloads::types::Tool,
    pub(crate) files: Vec<(String, Vec<u8>)>,
}

/// The `.py` files of the pinned corpus the python provider tests read, by name.
pub(crate) fn corpus_files() -> Result<Vec<(String, Vec<u8>)>, Box<dyn std::error::Error>> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/corpus");
    let mut files = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "py") {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or("a corpus file has no name")?;
            files.push((name.to_owned(), fs::read(&path)?));
        }
    }
    files.sort();
    Ok(files)
}

impl Inventory {
    pub(crate) fn stage() -> Result<Self, Box<dyn std::error::Error>> {
        let canonical = Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmarks/linters/ruff");
        let suite = suites::local();
        let profile = suite.profile(ArtifactPath(&canonical))?;
        assert_eq!(
            profile.defaults.len(),
            413,
            "the frozen Ruff default inventory"
        );
        let temporary = tempfile::tempdir()?;
        let staged = temporary.path().join("suite");
        let definitions = profile
            .defaults
            .iter()
            .chain(&profile.supporting_rules)
            .cloned()
            .collect();
        suite.stage(
            ArtifactPath(&canonical),
            ArtifactPath(&staged),
            &profile.defaults,
            &definitions,
            StagePurpose::Execution,
        )?;
        Ok(Self {
            temporary,
            staged,
            profile,
            files: corpus_files()?,
        })
    }

    /// The worker payload: all 413 rules over the corpus, with a fresh state directory.
    fn payload(&self) -> Result<String, Box<dyn std::error::Error>> {
        let suite = suites::local();
        let root = self.temporary.path().join("input");
        fs::create_dir_all(&root)?;
        let mut files = Vec::new();
        let mut tracked = Vec::new();
        for (name, bytes) in &self.files {
            fs::write(root.join(name), bytes)?;
            files.push(ryo::linter::RelativePath(name.as_str().into()));
            tracked.push(PathBuf::from(name));
        }
        let context = {
            use ryo_benchmark::outbound::context::interface::ContextPort as _;
            ryo_benchmark::outbound::context::factory::local().seal_existing(
                &root,
                &files,
                ryo_benchmark::outbound::context::types::CorpusFiles(&tracked),
            )?
        };
        let request = RunRequest {
            performance: PerformanceOptions { diagnostics: DiagnosticOptions::default() },
            selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(self.profile.defaults.clone()),
            rule_observations: ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
            rule_context: ryo_benchmark::domain::rule_context::service::resolve(
                &self.profile.rule_context,
                &ryo_benchmark_contracts::domain::workloads::types::ConfigurationOverride::Inherited,
            )?,
            project_search_roots: self.profile.project_search_roots.clone(),
            given_project_inventory: Default::default(),
            staged_sources: ryo_benchmark_contracts::domain::requests::types::StagedSources::Listed(files.clone()),
            file_metadata: ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Undeclared,
            context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Sealed(context),
            root,
            files,
            state: self.temporary.path().join("state"),
        };
        let candidate = linters::ryo_context(
            std::env::current_exe()?,
            self.staged.join("rules"),
            self.profile.query_modules.clone(),
            self.profile.rule_applicability.clone(),
            self.profile.parser_configuration.clone(),
            self.profile.source_language.clone(),
            self.profile.project_context_mode,
            self.profile.defaults.clone(),
            self.profile.rule_observation_selection.clone(),
            self.profile.rule_observations.clone(),
        );
        let invocation = candidate.prepare(&request)?;
        Ok(fs::read_to_string(Path::new(&invocation.arguments[2]))?)
    }

    /// The corpus under `configuration`, with its state directory cleared first so every configuration starts cold.
    pub(crate) fn run(
        &self,
        configuration: Configuration,
    ) -> Result<Measured, Box<dyn std::error::Error>> {
        let _cleared = fs::remove_dir_all(self.temporary.path().join("state"));
        measured(configuration, "corpus-413", self.payload()?)
    }
}

/// Whether a configuration unwires nothing, which is the baseline every other is compared with.
pub(crate) fn is_baseline(configuration: &Configuration) -> bool {
    configuration.policy == RoutePolicy::default()
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

/// Writes one row for each run into the results directory beside its README.
pub(crate) fn record(
    runs: &[Measured],
    baselines: &std::collections::BTreeMap<&'static str, Result<String, String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let rows: Vec<serde_json::Value> = runs
        .iter()
        .map(|run| {
            serde_json::json!({
                "configuration": run.configuration,
                "workload": run.workload,
                "identical_to_all_wired": baselines.get(run.workload) == Some(&run.output),
                "failed": run.output.is_err(),
                "output_sha256": digest(&run.output),
                "wall_seconds": run.wall_seconds,
                "cpu_seconds": run.cpu_seconds,
                "peak_rss_bytes": run.peak_rss_bytes,
            })
        })
        .collect();
    let target = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("benchmarks/linters/ruff/results/route-differential-2026-11-07/results.json");
    fs::write(target, serde_json::to_string_pretty(&rows)?)?;
    Ok(())
}
