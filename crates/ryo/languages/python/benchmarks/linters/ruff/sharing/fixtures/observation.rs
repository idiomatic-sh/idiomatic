//! The observation of one canonical run: the route table it is checked against and the census decoded from the trace events.
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
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
#[derive(Clone)]
pub(crate) struct Observation {
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
pub(crate) struct Route {
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
pub(crate) const MEANINGS: &str = include_str!(
    "../../../../../../../crates/execution/src/application/file_dag/program/facade_runtime/walk_coverage/fixtures/route_meanings.txt"
);
/// The meaning the shared table gives an operation kind.
pub(crate) fn meaning_of(kind: &str) -> Option<&'static str> {
    MEANINGS
        .lines()
        .filter(|line| !line.starts_with('#'))
        .find_map(|line| line.split_once(' ').filter(|(named, _)| *named == kind))
        .map(|(_, meaning)| meaning)
}
/// The routes an instruction of `kind` may bind to: a later instruction continues its region, an
/// observation union is an observation, and a head follows its kind's meaning.
pub(crate) fn expected_routes(kind: &str, head: bool) -> &'static [&'static str] {
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
pub(crate) fn observe() -> Result<Observation, Box<dyn std::error::Error>> {
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
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(
            profile.defaults.clone(),
        ),
        rule_observations:
            ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context: ryo_benchmark::domain::rule_context::service::resolve(
            &profile.rule_context,
            &ryo_benchmark_contracts::domain::workloads::types::ConfigurationOverride::Inherited,
        )?,
        project_search_roots: profile.project_search_roots.clone(),
        given_project_inventory: Default::default(),
        staged_sources: ryo_benchmark_contracts::domain::requests::types::StagedSources::Listed(
            files.clone(),
        ),
        file_metadata:
            ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Undeclared,
        context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Sealed(
            context,
        ),
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
pub(crate) struct Census {
    pub rules: Vec<String>,
    pub instructions: u64,
    pub physical_instructions: u64,
    pub shared: u64,
    pub classes: u64,
    pub barriers: [u64; 7],
}
#[derive(Clone)]
pub(crate) struct ReadyHead {
    pub logical: u64,
    pub relation: String,
    pub generation: String,
    pub population: String,
}
#[derive(Clone)]
pub(crate) struct Dispatch {
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
pub(crate) fn count(event: &serde_json::Value, field: &str) -> u64 {
    event[field].as_u64().expect("actual diagnostic counter")
}
pub(crate) fn decode(
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

pub(crate) fn population(fields: &serde_json::Value, require_proof: bool) -> String {
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
