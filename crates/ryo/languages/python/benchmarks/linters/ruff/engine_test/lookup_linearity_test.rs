//! The rules that resolve a name cost time and memory proportional to the file, not to its square, in every shape.
//!
//! The shapes are generated from one table: each operation (bind, rebind, import, handler, loop and with targets, walrus,
//! annotation, def, class, read, delete, `global` and `nonlocal` declarations, defaults, lambdas and comprehensions, many
//! functions reading a rebound module name, many distinct names, a read run longer than the first 14 pointer-jumping
//! rounds of `latest-before`) repeated N times in every scope kind it makes sense in (module, function, nested function,
//! class body, method). Each of F401, F841, F811 and F821 runs every program at 1,000, 2,000 and 4,000 repetitions; its
//! CPU time at each size must be at most `DOUBLING_LIMIT` times the time at the half size (a floor under the half-size
//! time keeps a fixed startup cost from deciding a ratio), and its peak resident memory at the largest size must stay
//! within `MEMORY_BASE_MB` plus `MEMORY_PER_REPETITION_KB` per repetition. A rule that joins every read to every
//! binding, pairs every effect with every other, or tests every handler against every read grows 3.5 to 8 times per
//! doubling and holds gigabytes.
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

/// The doubling of the file may cost at most this many times the time.
const DOUBLING_LIMIT: f64 = 2.2;
/// The half-size times below this many CPU seconds count as this many, so startup cost does not decide a ratio.
const TIME_FLOOR: f64 = 0.4;
/// The repetition counts of each shape.
const SIZES: [usize; 3] = [1000, 2000, 4000];
/// The memory every run may hold before its file counts.
const MEMORY_BASE_MB: f64 = 160.0;
/// The memory each repetition of a shape may add at the largest size.
const MEMORY_PER_REPETITION_KB: f64 = 40.0;

/// Where a shape's repetitions sit.
#[derive(Clone, Copy)]
enum Scope {
    Module,
    Function,
    Nested,
    Class,
    Method,
}

impl Scope {
    const ALL: [Scope; 5] = [Scope::Module, Scope::Function, Scope::Nested, Scope::Class, Scope::Method];

    fn label(self) -> &'static str {
        match self {
            Scope::Module => "module",
            Scope::Function => "function",
            Scope::Nested => "nested",
            Scope::Class => "class",
            Scope::Method => "method",
        }
    }
}

/// One operation: the lines before the repetitions and the lines repeated; `{i}` is the repetition's index.
struct Operation {
    name: &'static str,
    prologue: &'static str,
    repeat: &'static str,
    /// Only meaningful at module level (it defines the functions or defaults it reads there).
    module_only: bool,
    /// Needs an enclosing function binding (a `nonlocal` declaration).
    nested_only: bool,
}

const fn op(name: &'static str, prologue: &'static str, repeat: &'static str) -> Operation {
    Operation { name, prologue, repeat, module_only: false, nested_only: false }
}

const OPERATIONS: &[Operation] = &[
    op("assign", "", "v = 1\n"),
    op("assign-read", "", "v = 1\nprint(v)\n"),
    op("import", "", "import v\n"),
    op("import-read", "", "import v\nprint(v)\n"),
    op("handler", "", "try:\n    pass\nexcept Exception as v:\n    pass\n"),
    op("handler-read", "", "try:\n    pass\nexcept Exception as v:\n    print(v)\n"),
    op("for-target", "", "for v in y:\n    print(v)\n"),
    op("with-target", "", "with y as v:\n    print(v)\n"),
    op("walrus", "", "(v := 1)\nprint(v)\n"),
    op("annotation", "", "v: int\n"),
    op("annotation-read", "", "v: int\nprint(v)\n"),
    op("def", "", "def v():\n    pass\n"),
    op("class", "", "class v:\n    pass\n"),
    op("read", "v = 1\n", "print(v)\n"),
    op("assign-del", "", "v = 1\ndel v\n"),
    op("del", "v = 1\n", "del v\n"),
    op("global-declaration", "", "global v\n"),
    op("global-assign", "", "global v\nv = 1\n"),
    op("global-read", "", "global v\nprint(v)\n"),
    op("global-del", "", "global v\ndel v\n"),
    Operation { nested_only: true, ..op("nonlocal-assign", "", "nonlocal v\nv = 1\n") },
    op("distinct-import", "", "import v{i}\nprint(v{i})\n"),
    op("distinct-assign", "", "v{i} = 1\nprint(v{i})\n"),
    op("distinct-def", "", "def v{i}():\n    pass\n"),
    op("distinct-class", "", "class v{i}:\n    x = 1\n"),
    op("distinct-handler", "", "try:\n    pass\nexcept Exception as v{i}:\n    print(v{i})\n"),
    Operation { module_only: true, ..op("default-argument", "v = 1\n", "def g{i}(a=v):\n    pass\n") },
    Operation { module_only: true, ..op("rebind-read-by-functions", "", "v = 1\ndef g{i}():\n    return v\n") },
    Operation { module_only: true, ..op("lambda-read", "v = 1\n", "w{i} = lambda: v\n") },
    Operation { module_only: true, ..op("comprehension-read", "v = 1\n", "w{i} = [v for _ in y]\n") },
    op("method-read", "v = 1\n", "def m{i}(self):\n    return v\n"),
    Operation { nested_only: false, ..op("global-skipped-read", "global v\n", "v = 1\ndef g{i}():\n    return v\n") },
    op("long-read-run", "", "v = 1\n"),
];

/// A program: the operation repeated `count` times in the scope, or none when the pair makes no sense.
fn program(scope: Scope, operation: &Operation, count: usize) -> Option<String> {
    let repeated = |depth: usize| -> String {
        let pad = "    ".repeat(depth);
        let line = |text: &str| -> String {
            text.lines().map(|line| format!("{pad}{line}\n")).collect()
        };
        let mut text = line(operation.prologue);
        for index in 0..count {
            text.push_str(&line(&operation.repeat.replace("{i}", &index.to_string())));
        }
        if operation.name == "long-read-run" {
            text.push_str(&line(&"v\n".repeat(16_384 + count)));
        }
        text
    };
    match scope {
        _ if operation.module_only && !matches!(scope, Scope::Module) => None,
        _ if operation.nested_only && !matches!(scope, Scope::Nested) => None,
        Scope::Module => Some(repeated(0)),
        Scope::Function => Some(format!("def f():\n{}", repeated(1))),
        Scope::Nested => {
            let outer = if operation.nested_only { "    v = 1\n" } else { "" };
            Some(format!("def f():\n{outer}    def g():\n{}", repeated(2)))
        }
        Scope::Class => Some(format!("class C:\n{}", repeated(1))),
        Scope::Method => Some(format!("class C:\n    def m(self):\n{}", repeated(2))),
    }
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

/// The peak resident memory of this process since the last reset, in megabytes; zero where `/proc` is absent.
fn peak_resident_mb() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find(|line| line.starts_with("VmHWM:"))
                .and_then(|line| line.split_whitespace().nth(1)?.parse::<f64>().ok())
        })
        .map_or(0.0, |kilobytes| kilobytes / 1024.0)
}

/// Forget the peak so far, so the next reading is the next run's own.
fn reset_peak_resident() {
    let _ignored = std::fs::write("/proc/self/clear_refs", "5");
}

/// Every generated program at every size: no doubling may cost more than the limit and no run may hold more than its bound.
pub(super) fn rule_cost_grows_with_the_file(rule: &str) -> Result<(), Error> {
    let mut failures = Vec::new();
    for scope in Scope::ALL {
        for operation in OPERATIONS {
            if program(scope, operation, 1).is_none() {
                continue;
            }
            let mut seconds = Vec::new();
            let mut peaks = Vec::new();
            for size in SIZES {
                let text = program(scope, operation, size).unwrap_or_default();
                reset_peak_resident();
                let (elapsed, _report) = timed(rule, &text)?;
                seconds.push(elapsed);
                peaks.push(peak_resident_mb());
            }
            let ratios: Vec<f64> = seconds
                .windows(2)
                .map(|pair| pair[1] / pair[0].max(TIME_FLOOR))
                .collect();
            let bound = MEMORY_BASE_MB + MEMORY_PER_REPETITION_KB * SIZES[2] as f64 / 1024.0;
            let name = format!("{rule} {} {}", scope.label(), operation.name);
            if ratios.iter().any(|ratio| *ratio > DOUBLING_LIMIT) {
                failures.push(format!("{name}: seconds {seconds:.2?} at {SIZES:?}, ratios {ratios:.2?}"));
            }
            if peaks[2] > bound {
                failures.push(format!("{name}: peak {:.0} MB at {} repetitions, bound {bound:.0} MB", peaks[2], SIZES[2]));
            }
        }
    }
    assert!(failures.is_empty(), "each doubling may cost at most {DOUBLING_LIMIT}x: {failures:#?}");
    Ok(())
}
