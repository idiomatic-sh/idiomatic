//! Load canonical selected queries through existing production artifact and worker boundaries.
use ryo_benchmark::inbound::use_cases::run_ryo::{
    interface::RunRyoUseCase, use_case::RunRyoUseCaseImpl,
};
use ryo_benchmark::outbound::{
    engine::factory,
    linter::{factory as linters, interface::LinterPort},
    suite::{factory as suites, interface::SuitePort, types::ArtifactPath},
    worker::factory as workers,
};
use ryo_benchmark_contracts::domain::{
    requests::types::{DiagnosticOptions, PerformanceOptions, RunRequest},
    workloads::types::StagePurpose,
};
use ryo::linter::{RouteWiring, RoutePolicy};
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Instant,
};

#[derive(Clone)]
pub(super) struct Observation {
    pub selected: BTreeSet<String>,
    pub census: Census,
    pub classes: std::collections::BTreeMap<u64, String>,
    /// The rules whose compiled instructions are heads of each class.
    pub class_rules: std::collections::BTreeMap<u64, BTreeSet<u64>>,
    pub dispatches: Vec<Dispatch>,
    pub heads: std::collections::BTreeMap<u64, u64>,
    pub ready: Vec<ReadyHead>,
    pub refused: BTreeSet<(String, String, u64)>,
    /// The route the walk bound each physical instruction to, as the census traced it.
    pub routes: Vec<Route>,
}
#[derive(Clone)]
pub(super) struct Route {
    pub logical: u64,
    pub kind: String,
    pub head: bool,
    /// The partition of the instruction's region: "PerFile" or "CrossFile".
    pub phase: String,
    pub route: String,
    pub class: Option<u64>,
}
// The route each operation kind's meaning allows. Execution's route test checks this table against its
// `meaning` for every operation kind, so the two cannot drift.
const MEANINGS: &str = include_str!(
    "../../../../../../../crates/execution/src/domain/shared_walk/types/prepared_walk/value/coverage/fixtures/route_meanings.txt"
);
/// The meaning the shared table gives an operation kind.
fn meaning_of(kind: &str) -> Option<&'static str> {
    MEANINGS
        .lines()
        .filter(|line| !line.starts_with('#'))
        .find_map(|line| line.split_once(' ').filter(|(named, _)| *named == kind))
        .map(|(_, meaning)| meaning)
}
/// The routes an instruction of `kind` may bind to: a later instruction continues its region, an
/// observation union is an observation, and a head follows its kind's meaning.
pub(super) fn expected_routes(kind: &str, head: bool) -> &'static [&'static str] {
    if !head {
        return &["Continuation", "Observation"];
    }
    match meaning_of(kind) {
        Some("selection") => &["Shared", "Representation", "DependentInput", "Observation"],
        Some("per-row") => &["RowSemantics", "Observation"],
        Some("whole-input") => &["DerivedOrOrdered", "Observation"],
        Some("request-owned") => &["RequestOwnedSource", "Observation"],
        Some(other) => panic!("operation kind {kind} has unknown meaning {other}"),
        None => panic!("operation kind {kind} has no declared meaning"),
    }
}
pub(super) fn observe() -> Result<Observation, Box<dyn std::error::Error>> {
    let canonical = Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmarks/linters/ruff");
    let suite = suites::local();
    let profile = suite.profile(ArtifactPath(&canonical))?;
    // Independent canonical registration, not reconstructed from engine census.
    let selected = profile
        .defaults
        .iter()
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    assert_eq!(selected.len(), 413, "actual frozen Ruff default inventory");
    assert_eq!(
        selected.len(),
        profile.defaults.len(),
        "no duplicate canonical IDs"
    );
    let temporary = tempfile::tempdir()?;
    let staged = temporary.path().join("suite");
    // Stage the whole declared profile; the production worker resolves which
    // supporting definitions are needed by the selected diagnostic cohort.
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
    let root = temporary.path().join("input");
    fs::create_dir(&root)?;
    fs::write(
        root.join("case.py"),
        "import os\n\ndef identity(value):\n    return value\n",
    )?;
    // A second file with different content: its file generation and admitted populations
    // must be fresh, never reused from the first.
    fs::write(
        root.join("second.py"),
        "class Second:\n    def method(self, value):\n        return value + 1\n",
    )?;
    let files = vec![
        ryo::linter::RelativePath("case.py".into()),
        ryo::linter::RelativePath("second.py".into()),
    ];
    let tracked = vec![
        std::path::PathBuf::from("case.py"),
        std::path::PathBuf::from("second.py"),
    ];
    let context = {
        use ryo_benchmark::outbound::context::interface::ContextPort as _;
        ryo_benchmark::outbound::context::factory::local().seal_existing(
            &root,
            &files,
            ryo_benchmark::outbound::context::types::CorpusFiles(&tracked),
        )?
    };
    let request = RunRequest {
        performance: PerformanceOptions {
            diagnostics: DiagnosticOptions {
                execution: ryo_benchmark_contracts::domain::requests::types::DiagnosticSwitch::On,
                ..Default::default()
            },
            ..Default::default()
        },
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(profile.defaults.clone()),
        rule_observations: ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context: ryo_benchmark::domain::rule_context::service::resolve(
            &profile.rule_context,
            &ryo_benchmark_contracts::domain::workloads::types::ConfigurationOverride::Inherited,
        )?,
        project_search_roots: profile.project_search_roots.clone(),
        given_project_inventory: Default::default(),
        staged_sources: ryo_benchmark_contracts::domain::requests::types::StagedSources::Listed(files.clone()),
        file_metadata: ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Undeclared,
        context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Sealed(context),
        root,
        files,
        state: temporary.path().join("state"),
    };
    let candidate = linters::ryo_context(
        std::env::current_exe()?,
        staged.join("rules"),
        profile.query_modules,
        profile.rule_applicability,
        profile.parser_configuration,
        profile.source_language,
        profile.project_context_mode,
        profile.defaults,
        profile.rule_observation_selection,
        profile.rule_observations,
    );
    let invocation = candidate.prepare(&request)?;
    let payload = fs::read_to_string(Path::new(&invocation.arguments[2]))?;
    let worker = workers::json(payload.into(), Box::new(std::io::sink()));
    let destination = temporary.path().join("events.jsonl");
    let engine = factory::local_with_diagnostic_events(
        ryo::linter::Languages(vec![ryo_python::linked_with_settings(
            ryo_python::settings::PythonParserSettings::new(
                ryo_python::settings::PythonTargetVersion::Py310,
            ),
        )]),
        destination.clone(),
        ryo_benchmark::outbound::engine::types::FrontendDiagnosticProjection {
            rule: ryo::linter::RuleId::try_from("invalid-syntax")?,
            query: ryo::query::RyoQlQuery::try_from(
                crate::frontend_projection::constants::FRONTEND_DIAGNOSTIC_QUERY,
            )?,
        },
    );
    RunRyoUseCaseImpl::new(
        std::sync::Arc::new(engine),
        std::sync::Arc::new(worker),
        std::sync::Arc::new(ryo_benchmark::outbound::context::factory::local()),
    )
    .run_ryo()?;
    decode(selected, &fs::read_to_string(destination)?)
}

#[derive(Clone)]
pub(super) struct Census {
    pub rules: Vec<String>,
    pub instructions: u64,
    pub physical_instructions: u64,
    pub shared: u64,
    pub classes: u64,
    pub barriers: [u64; 7],
}
#[derive(Clone)]
pub(super) struct ReadyHead {
    pub logical: u64,
    pub relation: String,
    pub generation: String,
    pub population: String,
}
#[derive(Clone)]
pub(super) struct Dispatch {
    pub generation: String,
    /// The partition whose source the dispatch walked: "PerFile" or "CrossFile".
    pub phase: String,
    pub relation: String,
    pub population: String,
    pub classes: Vec<u64>,
    pub result_slots: u64,
    pub passes: u64,
    pub source_rows: u64,
    pub decoded_rows: u64,
}
fn count(event: &serde_json::Value, field: &str) -> u64 {
    event[field].as_u64().expect("actual diagnostic counter")
}
fn decode(
    selected: BTreeSet<String>,
    lines: &str,
) -> Result<Observation, Box<dyn std::error::Error>> {
    let mut census = None;
    let mut classes = std::collections::BTreeMap::new();
    let mut class_rules = std::collections::BTreeMap::new();
    let mut dispatches = Vec::new();
    let mut heads = std::collections::BTreeMap::new();
    let mut ready = Vec::new();
    let mut refused = BTreeSet::new();
    let mut routes = Vec::new();
    for line in lines.lines() {
        let event: serde_json::Value = serde_json::from_str(line)?;
        let fields = &event["fields"];
        let Some(name) = fields["event"].as_str() else {
            continue;
        };
        if !name.starts_with("shared_walk_") {
            continue;
        }
        match name {
            "shared_walk_census" => {
                let actual = Census {
                    rules: serde_json::from_str(fields["rules"].as_str().unwrap())?,
                    instructions: count(fields, "instructions"),
                    physical_instructions: count(fields, "physical_instructions"),
                    shared: count(fields, "shared"),
                    classes: count(fields, "classes"),
                    barriers: [
                        "representation",
                        "dependent",
                        "observation",
                        "row",
                        "derived",
                        "request",
                        "continuation",
                    ]
                    .map(|reason| count(fields, reason)),
                };
                assert!(census.replace(actual).is_none(), "duplicate request census");
            }
            "shared_walk_class" => {
                for head in serde_json::from_str::<Vec<u64>>(fields["heads"].as_str().unwrap())? {
                    assert!(
                        heads.insert(head, count(fields, "class")).is_none(),
                        "one compiled class per physical head"
                    );
                }
                assert!(
                    class_rules
                        .insert(
                            count(fields, "class"),
                            serde_json::from_str::<BTreeSet<u64>>(
                                fields["rules"].as_str().unwrap()
                            )?
                        )
                        .is_none(),
                    "duplicate compiled class membership"
                );
                assert!(
                    classes
                        .insert(
                            count(fields, "class"),
                            fields["relation"].as_str().unwrap().to_owned()
                        )
                        .is_none(),
                    "duplicate compiled class"
                );
            }
            "shared_walk_route" => routes.push(Route {
                logical: count(fields, "logical"),
                kind: fields["kind"].as_str().unwrap().to_owned(),
                head: fields["head"].as_bool().unwrap(),
                phase: fields["phase"].as_str().unwrap().to_owned(),
                route: fields["barrier"].as_str().unwrap_or("Shared").to_owned(),
                class: fields["class"].as_u64(),
            }),
            "shared_walk_ready" => {
                assert_eq!(fields["phase"].as_str(), Some("PerFile"));
                ready.push(ReadyHead {
                    logical: count(fields, "logical"),
                    relation: fields["relation"].as_str().unwrap().to_owned(),
                    generation: fields["generation"].as_str().unwrap().to_owned(),
                    population: population(fields, true),
                });
            }
            "shared_walk_refused" => {
                assert_eq!(
                    fields["reason"].as_str(),
                    Some("Representation"),
                    "only the actual representation admission guard can discharge an obligation"
                );
                let generation = fields["generation"].as_str().unwrap().to_owned();
                let population = population(fields, false);
                for class in serde_json::from_str::<Vec<u64>>(fields["classes"].as_str().unwrap())?
                {
                    // Equivalent ready heads may independently reach the same
                    // authoritative representation refusal; identity is already bound.
                    refused.insert((generation.clone(), population.clone(), class));
                }
            }
            "shared_walk_dispatch" => dispatches.push(Dispatch {
                generation: fields["generation"].as_str().unwrap().to_owned(),
                phase: fields["phase"].as_str().unwrap().to_owned(),
                relation: fields["relation"].as_str().unwrap().to_owned(),
                population: fields["population"].as_str().unwrap().to_owned(),
                classes: serde_json::from_str(fields["classes"].as_str().unwrap())?,
                result_slots: count(fields, "result_slots"),
                passes: count(fields, "passes"),
                source_rows: count(fields, "source_rows"),
                decoded_rows: count(fields, "decoded_rows"),
            }),
            _ => panic!("unaccounted shared-walk event: {fields}"),
        }
    }
    Ok(Observation {
        selected,
        census: census.expect("actual bound program census"),
        classes,
        class_rules,
        dispatches,
        heads,
        ready,
        refused,
        routes,
    })
}

fn population(fields: &serde_json::Value, require_proof: bool) -> String {
    let population = fields["population"]
        .as_str()
        .expect("actual admitted population");
    if let Some(proof) = population
        .strip_prefix("Ok(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return proof.to_owned();
    }
    assert!(
        !require_proof && population.starts_with("PopulationId("),
        "runtime admission proof must succeed: {fields}"
    );
    population.to_owned()
}

/// The Django canary request the runbook names: the sealed corpus and the cold request that lists its files.
pub(super) const DJANGO_REQUEST: &str = "/tmp/ryo-event-ready-django213-20261105/cold-request.json";
/// The one rule of the Django canary (112 diagnostics at the runbook baseline).
pub(super) const DJANGO_RULE: &str = "N999";

/// One engine configuration: a name and the strategies it wires.
#[derive(Clone, Copy, Debug)]
pub(super) struct Configuration {
    pub name: &'static str,
    pub policy: RoutePolicy,
}

/// The baseline, each strategy unwired alone, and every strategy unwired: the generic evaluation of the operators.
pub(super) fn configurations() -> Vec<Configuration> {
    let all = RoutePolicy::default();
    let off = RouteWiring::Unwired;
    vec![
        Configuration { name: "all-wired", policy: all },
        Configuration {
            name: "without-ordered-existence",
            policy: RoutePolicy { ordered_existence: off, ..all },
        },
        Configuration {
            name: "without-column-kernel",
            policy: RoutePolicy { column_kernel: off, ..all },
        },
        Configuration {
            name: "without-stored-selection",
            policy: RoutePolicy { stored_selection: off, ..all },
        },
        Configuration {
            name: "without-native-positions",
            policy: RoutePolicy { native_positions: off, ..all },
        },
        Configuration {
            name: "without-tree-join",
            policy: RoutePolicy { tree_join: off, ..all },
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
pub(super) struct Measured {
    pub configuration: &'static str,
    pub workload: &'static str,
    /// The worker's report text, or the error text: compared byte for byte with the baseline's.
    pub output: Result<String, String>,
    pub wall_seconds: f64,
    pub cpu_seconds: Option<f64>,
    pub peak_rss_bytes: Option<u64>,
}

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);
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

fn process_cpu_seconds() -> Option<f64> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let after = stat.rsplit_once(')')?.1;
    let fields: Vec<&str> = after.split_whitespace().collect();
    let ticks = fields.get(11)?.parse::<f64>().ok()? + fields.get(12)?.parse::<f64>().ok()?;
    Some(ticks / 100.0)
}

fn peak_rss_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmHWM:"))?;
    let kilobytes = line.split_whitespace().nth(1)?.parse::<u64>().ok()?;
    Some(kilobytes * 1024)
}

/// Resets the peak resident size so the next reading belongs to the next run alone.
fn reset_peak() {
    let _ignored: std::io::Result<()> = std::fs::write("/proc/self/clear_refs", "5");
}

/// Runs the worker payload through the engine of `configuration`, measuring the run.
fn measured(
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
    let cpu_seconds = process_cpu_seconds().zip(cpu).map(|(after, before)| after - before);
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
pub(super) fn django(
    configuration: Configuration,
) -> Result<Measured, Box<dyn std::error::Error>> {
    let mut request: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(DJANGO_REQUEST)?)?;
    let cache = tempfile::tempdir()?;
    request["selected_rules"] = serde_json::json!([DJANGO_RULE]);
    request["cache"] = serde_json::json!(cache.path().join("cache"));
    // The request predates the retired strategy selector: its one removed key is dropped, nothing else is changed.
    if let Some(performance) = request["performance"].as_object_mut() {
        let _removed = performance.remove("column_kernel");
    }
    measured(configuration, "django-n999", serde_json::to_string(&request)?)
}

/// The staged 413-rule profile and the pinned real-code corpus, prepared once for every configuration.
pub(super) struct Inventory {
    temporary: tempfile::TempDir,
    staged: PathBuf,
    profile: ryo_benchmark_contracts::domain::workloads::types::Tool,
    files: Vec<(String, Vec<u8>)>,
}

/// The `.py` files of the pinned corpus the python provider tests read, by name.
fn corpus_files() -> Result<Vec<(String, Vec<u8>)>, Box<dyn std::error::Error>> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/corpus");
    let mut files = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "py") {
            let name = path.file_name().and_then(|name| name.to_str()).ok_or("a corpus file has no name")?;
            files.push((name.to_owned(), fs::read(&path)?));
        }
    }
    files.sort();
    Ok(files)
}

impl Inventory {
    pub(super) fn stage() -> Result<Self, Box<dyn std::error::Error>> {
        let canonical = Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmarks/linters/ruff");
        let suite = suites::local();
        let profile = suite.profile(ArtifactPath(&canonical))?;
        assert_eq!(profile.defaults.len(), 413, "the frozen Ruff default inventory");
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
        Ok(Self { temporary, staged, profile, files: corpus_files()? })
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
    pub(super) fn run(
        &self,
        configuration: Configuration,
    ) -> Result<Measured, Box<dyn std::error::Error>> {
        let _cleared = fs::remove_dir_all(self.temporary.path().join("state"));
        measured(configuration, "corpus-413", self.payload()?)
    }
}

/// Whether a configuration unwires nothing, which is the baseline every other is compared with.
pub(super) fn is_baseline(configuration: &Configuration) -> bool {
    configuration.policy == RoutePolicy::default()
}

fn digest(output: &Result<String, String>) -> String {
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
pub(super) fn record(
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
