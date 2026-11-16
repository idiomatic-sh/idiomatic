//! The rules that resolve a name cost time proportional to the file, not to its square.
//!
//! A file of N pairs `x = i; print(x)` has N bindings of one name and N reads of it, and a file of N `def f(): pass` has N
//! redefinitions of one name. A lookup that joins every read to every earlier binding, or a rule that pairs every binding
//! with every later one, grows 3.5 to 6 times per doubling and holds gigabytes; one ordered pass doubles. Each of F401, F841,
//! F811 and F821 runs over both shapes at 1,000, 2,000 and 4,000 repetitions, and its time at each size must be at most
//! `DOUBLING_LIMIT` times the time at the half size. A name read more times between two bindings than one pointer-jumping
//! pass spans takes the exact fallback of the shared `latest-before` fragment; the last proof reads one name 17,000 times
//! and requires that F821 finds every one of them bound, as it does for a short chain.
use super::prepared_engine_test::{
    PreparedEngine as _, default_frontend_projection, fixture_cohort, module_names, python_310,
};
use cpu_time::ProcessTime;
use ryo_benchmark::outbound::engine::factory;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{ExecutionMetadata, RunReport};
use ryo_benchmark_contracts::domain::requests::types::{
    ApplicabilityPolicy, CohortSelection, ContextSeal, InputDeclaration, ParserConfigurationName,
    ParserSelection, ProviderSelection, RuleContextPreparation, RuleLabel, SearchRoots,
    WorkerRequest, WorkerState,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The doubling of the file may at most this many times the time.
const DOUBLING_LIMIT: f64 = 2.2;
/// The repetition counts of each shape.
const SIZES: [usize; 3] = [1000, 2000, 4000];
/// The runs of one size whose cheapest is the time; the process's CPU seconds, so a busy machine does not decide it.
const RUNS: usize = 2;
/// The rules whose cost the lookup or their own pair enumeration decides.
const RULES: [&str; 4] = ["F401", "F841", "F811", "F821"];
/// More reads of one name between two bindings than the pointer-jumping rounds of `latest-before` span.
const LONG_CHAIN: usize = 17_000;

/// The shapes of a file of `count` repetitions: reads that each follow a binding, and bindings that each redefine one.
fn shapes(count: usize) -> [(&'static str, String); 2] {
    [
        (
            "pairs",
            (0..count).map(|index| format!("x = {index}\nprint(x)\n")).collect(),
        ),
        ("redefinitions", "def f():\n    pass\n".repeat(count)),
    ]
}

/// The modules the rule's profile registers, resolved against this bench's directory.
fn registered_modules(rule: &str) -> Result<BTreeMap<String, PathBuf>, Error> {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(source.join(format!("profiles/{rule}/benchmark.toml")))?;
    let profile: toml::Value =
        toml::from_str(&text).map_err(|error| Error::Invalid(error.to_string()))?;
    let modules = profile
        .get("query-modules")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| Error::Invalid(format!("{rule} profile declares no modules")))?;
    modules
        .iter()
        .map(|(name, path)| {
            path.as_str()
                .map(|path| (name.clone(), PathBuf::from(path)))
                .ok_or_else(|| Error::Invalid(format!("{rule} module path is not text")))
        })
        .collect()
}

/// Run the rule's real query and registered modules over one file, and say the CPU seconds the run took.
fn timed(rule: &str, text: &str) -> Result<(f64, RunReport), Error> {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = tempfile::tempdir()?;
    let modules = registered_modules(rule)?;
    for path in modules.values() {
        let destination = root.path().join(path);
        let parent = destination
            .parent()
            .ok_or_else(|| Error::Invalid("module has no parent".into()))?;
        std::fs::create_dir_all(parent)?;
        let _copied = std::fs::copy(source.join(path), destination)?;
    }
    let rules = root.path().join("rules");
    let directory = rules.join(rule);
    std::fs::create_dir_all(&directory)?;
    let _copied = std::fs::copy(
        source.join(format!("rules/{rule}/query.ryoql")),
        directory.join("query.ryoql"),
    )?;
    std::fs::write(root.path().join("case.py"), text)?;
    let request = WorkerRequest {
        performance: Default::default(),
        selected_rules: CohortSelection::Explicit(fixture_cohort(&[rule])),
        rule_observations: InputDeclaration::Undeclared,
        rule_context: RuleContextPreparation::Unconfigured,
        project_search_roots: SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        project_context_mode: ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        context_manifest: ContextSeal::Unsealed,
        root: root.path().to_owned(),
        rules,
        query_modules: module_names(modules),
        rule_applicability: BTreeMap::from([(
            RuleLabel(rule.into()),
            ApplicabilityPolicy("accepted-syntax".into()),
        )]),
        parser_configuration: ParserSelection::Named(ParserConfigurationName(
            "python-target=3.10".into(),
        )),
        source_language: ProviderSelection::Unspecified,
        files: vec![ryo::linter::RelativePath("case.py".into())],
        state: WorkerState::Fresh,
    };
    let engine =
        factory::local_with_frontend_diagnostic(python_310(), None, default_frontend_projection()?);
    let started = ProcessTime::now();
    let report = engine.run_prepared(request)?;
    let elapsed = started.elapsed().as_secs_f64();
    assert_eq!(report.execution_metadata, ExecutionMetadata::Complete, "{rule}");
    Ok((elapsed, report))
}

/// Each rule over each shape at each size; no doubling may cost more than the limit.
pub(super) fn lookup_rules_cost_grows_with_the_file_not_its_square() -> Result<(), Error> {
    let mut slow = Vec::new();
    for rule in RULES {
        for shape in 0..2 {
            let mut name = "";
            let mut seconds = Vec::new();
            for size in SIZES {
                let (label, text) = &shapes(size)[shape];
                name = label;
                let mut best = f64::MAX;
                for _ in 0..RUNS {
                    best = best.min(timed(rule, text)?.0);
                }
                seconds.push(best);
            }
            let ratios: Vec<f64> = seconds.windows(2).map(|pair| pair[1] / pair[0]).collect();
            if ratios.iter().any(|ratio| *ratio > DOUBLING_LIMIT) {
                slow.push(format!(
                    "{rule} {name}: seconds at {SIZES:?} are {seconds:?}, doubling ratios {ratios:?}"
                ));
            }
        }
    }
    assert!(
        slow.is_empty(),
        "each doubling may cost at most {DOUBLING_LIMIT}x: {slow:#?}"
    );
    a_name_read_beyond_one_pass_still_resolves()
}

/// One name read more times than one pointer-jumping pass spans still resolves to its binding.
fn a_name_read_beyond_one_pass_still_resolves() -> Result<(), Error> {
    let long_reads = |prologue: &str, read: &str| format!("{prologue}{}", read.repeat(LONG_CHAIN));
    for (rule, text) in [("F821", long_reads("x = 1\n", "x\n"))] {
        let (_, report) = timed(rule, &text)?;
        assert!(
            report.diagnostics.is_empty(),
            "{rule}: the {LONG_CHAIN} reads must resolve to the binding: {:?}",
            report.diagnostics.first()
        );
    }
    Ok(())
}
