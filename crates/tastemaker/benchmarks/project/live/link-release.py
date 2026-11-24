"""Link the candidate in a fresh archive of the benchmark's pinned Tastemaker release."""
from pathlib import Path
import sys
import json

release = Path(sys.argv[1]).resolve()
library = Path(__file__).resolve().parent
core = release / 'crates/tastemaker/crates/core'
manifest = core / 'Cargo.toml'
s = manifest.read_text()
old = '[dependencies]\n'
assert s.count(old) == 1 and 'tm-project-session' not in s
manifest.write_text(s.replace(old, old + f'tm-project-session = {{ path = "{library}" }}\n'))
workspace = release / "crates/tastemaker/Cargo.toml"
cfg = workspace.read_text()
k = "[patch.crates-io]\n"
assert cfg.count(k) == 1
# Keep the released public types (including their non-exhaustive fields) and
# replace the lint resource only in the benchmark's SDK copy.
sdk_src = release / "crates/ryo/crates/sdk/src"
image_sources = library.parents[3] / "ryo/crates/engine/benches/lint_image"
(sdk_src / "linked_sdk.rs").write_text(
    f'include!({json.dumps(str(library / "linked-sdk.rs"))});\n'
    f'include!({json.dumps(str(image_sources / "sdk_bundle.rs"))});\n'
)
sdk_lib = sdk_src / "lib.rs"
sdk_lib.write_text(
    '#![allow(dead_code, reason = "the linked benchmark replaces the shipped lint implementation")]\n' +
    sdk_lib.read_text() + "\n#[doc(hidden)]\npub mod linked_sdk;\n"
)
linter = sdk_src / "resources/linter.rs"
s = linter.read_text().replace("pub mod lint;", "pub(crate) mod lint;")
s = s.replace("pub use crate::domain::capacity::types::MemoryBudget;\n\n", "")
s = s.replace("pub use lint::entry::lint;\n\n", "").replace("pub use lint::entry::lint_with;\n\n", "")
s = s.replace("pub use lint::types::Linter;", "pub use crate::linked_sdk::Linter;")
linter.write_text(s)
# query::run still accepts this type. Keep it nameable in its owning resource.
with (sdk_src / "resources/query.rs").open("a") as f:
    f.write("\npub use crate::domain::capacity::types::MemoryBudget;\n")
lint = sdk_src / "resources/linter/lint"
entry = lint / "entry.rs"
s = entry.read_text()
workers = s[s.index("/// The threads every run of the caller"):]
entry.write_text("""//! Construction of the SDK's worker pool.
use super::{schemas, types::Workers};
use crate::domain::capacity::types::WorkerPool;

""" + workers)
types = lint / "types.rs"
s = types.read_text()
start = s.index("/// The engine a program builds once")
end = s.index("/// What every run over one linter shares", start)
s = s[:start] + s[end:]
start = s.index("impl Linter {")
end = s.index("/// The rules to check in a repository.", start)
types.write_text(s[:start] + s[end:])
workspace.write_text(cfg.replace(k, k +
    'idiomatic-sh-ryo-relations = { path = "../ryo/crates/relations" }\n'))
# Keep SDK calls on a single scheduler thread. The in-process session owns its workers.
for operation in ('check', 'validate'):
    connections = release / f'crates/tastemaker/cli/src/resources/workspace/{operation}/connections.rs'
    source = connections.read_text()
    chooser = 'std::thread::available_parallelism().map_or(1, std::num::NonZero::get)'
    assert source.count(chooser) == 1
    connections.write_text(source.replace(chooser, '1'))

adapter = core / 'src/outbound/analysis/adapters/ryo/adapter.rs'
s = adapter.read_text()
s = s.replace("`ryo::linter::lint_with`", "`ryo::linter::Linter::lint`")
s = s.replace("InvalidReason, Memory, QueryVerdict", "InvalidReason, QueryVerdict")
start = s.index("    fn lint(&self,")
end = s.index("    fn relations(", start)
constructor = s[s.index("        let linter = self", start):s.index("        ryo::linter::lint_with", start)]
constructor = constructor.replace(".map_err(Clone::clone)?;", ".map_err(Clone::clone);")
memory = constructor.index("                    match self.settings.memory")
constructor = constructor[:memory] + constructor[constructor.index("                    self.cache.clone(),", memory):]
s = s[:start] + """    fn lint(&self, request: AnalysisRequest) -> Result<AnalysisReport, AnalysisError> {
        self.lint_benchmark_round(vec![request]).pop().expect("one result")
    }

    fn benchmark_round_capacity(&self) -> usize {
        // Bounds pending fixture continuations independently of the SDK.
        16
    }

    fn lint_benchmark_round(
        &self, requests: Vec<AnalysisRequest>,
    ) -> Vec<Result<AnalysisReport, AnalysisError>> {
""" + constructor + """
        let linter = match linter {
            Ok(linter) => linter,
            Err(reason) => return requests.into_iter().map(|_| Err(reason.clone())).collect(),
        };
        let mut positions = Vec::new();
        let mut valid = Vec::new();
        let mut answers = Vec::with_capacity(requests.len());
        for (index, result) in requests.into_iter().map(schemas::request).enumerate() {
            match result {
                Ok(request) => {
                    positions.push(index);
                    valid.push(request);
                    answers.push(None);
                }
                Err(reason) => answers.push(Some(Err(reason))),
            }
        }
        let results = tm_project_session::measure_dispatch(|| linter.lint(valid));
        assert_eq!(results.len(), positions.len());
        for (index, result) in positions.into_iter().zip(results) {
            answers[index] = Some(result.map(|value| { let start = std::time::Instant::now(); let answer = report(value); tm_project_session::host_wall(tm_project_session::HostPhase::Conversion, start.elapsed().as_secs_f64()); answer }).map_err(error));
        }
        answers.into_iter().map(Option::unwrap).collect()
    }

""" + s[end:]
adapter.write_text(s)


def link_capture(release, library):
    cli = release / 'crates/tastemaker/cli'
    manifest = cli / 'Cargo.toml'
    s = manifest.read_text()
    deps = '[dependencies]\n'
    assert s.count(deps) == 1
    manifest.write_text(s.replace(deps, deps + f'rayon.workspace = true\ntm-project-session = {{ path = "{library}" }}\n'))
    main = cli / 'src/main.rs'
    s = main.read_text()
    declaration = 'fn main() -> std::process::ExitCode {'
    assert s.count(declaration) == 1
    main.write_text(s.replace(declaration,'''#[path = "languages.rs"]
mod benchmark_languages;

fn main() -> std::process::ExitCode {
    let mut arguments = std::env::args().skip(1);
    let mode = arguments.next();
    if mode.as_deref() == Some("--mixed") {
        let pack=arguments.next().expect("pack");
        let input=arguments.next().expect("input");
        let output=arguments.next().expect("output");
        if let Err(reason) = tm_project_session::capture_one(benchmark_languages::linked(), &pack, &input, &output) {
            eprintln!("{reason}");
            return std::process::ExitCode::FAILURE;
        }
        return std::process::ExitCode::SUCCESS;
    }
    if mode.as_deref() == Some("--tm-batch-retained") {
        let root=arguments.next().expect("store root");
        let manifest=arguments.next().expect("manifest");
        let output=arguments.next().expect("output");
        tm_project_session::capture_retained(benchmark_languages::linked(),&manifest,&output,&root);
        return std::process::ExitCode::SUCCESS;
    }
    if mode.as_deref() == Some("--tm-batch") {
        let manifest = arguments.next().expect("manifest");
        let output = arguments.next().expect("output");
        let workers = arguments.next().map(|flag| {
            assert_eq!(flag, "--workers");
            arguments.next().expect("worker count").parse().expect("positive worker count")
        });
        assert!(arguments.next().is_none(), "unexpected batch argument");
        // The one-worker replay enters from main. Let linked project jobs execute
        // there so its phase clock accounts for the entire check.
        if workers.or_else(|| std::thread::available_parallelism().ok())
            .is_none_or(|n| n.get() == 1) {
            rayon::ThreadPoolBuilder::new().num_threads(1).use_current_thread()
                .build_global().expect("initialize one-worker replay");
        }
        tm_project_session::capture_then(benchmark_languages::linked(), &manifest, &output, workers, || {
            std::io::Write::flush(&mut std::io::stdout()).expect("flush capture log");
            std::process::exit(0);
        });
        return std::process::ExitCode::SUCCESS;
    }'''))

link_capture(release, library)

entry = release / "crates/tastemaker/cli/src/main.rs"
s = entry.read_text()
original = '    tastemaker_cli::resources::run(&start, &build.0)'
assert s.count(original) == 1
entry.write_text(s.replace(original, '    tm_project_session::link_executor();\n    let result = tastemaker_cli::resources::run(&start, &build.0);\n    tm_project_session::print_measurement();\n    result'))

cli=release/'crates/tastemaker/cli/src'
lib=cli/'lib.rs'
with lib.open('a') as out:
    out.write('\n/// The check and the capture use the same linked reader instances.\npub fn benchmark_languages() -> ryo::linter::Languages { languages::linked() }\n')
main=cli/'main.rs'
s=main.read_text().replace('#[path = "languages.rs"]\nmod benchmark_languages;\n\n','').replace('benchmark_languages::linked()', 'tastemaker_cli::benchmark_languages()')
main.write_text(s)


# The pinned command supplies typed options and accounts separately for host work.
command = cli / 'resources/workspace/check/command.rs'
s = command.read_text()
field = '    pub(super) fix: bool,'
assert s.count(field) == 1
s = s.replace(field, field + """
    /// Engine workers for this SDK check.
    #[arg(long)]
    pub(super) workers: Option<std::num::NonZeroUsize>,
    /// Reverse the source list entering the SDK, for order checks.
    #[arg(long, hide = true)]
    pub(super) reverse_files: bool,
""")
edge = '    let request = super::schemas::request(arguments.paths,'
assert s.count(edge) == 1
s = s.replace(edge, """    tm_project_session::configure_bench_check(
        arguments.workers, arguments.reverse_files,
    );
""" + edge)
s = s.replace('    proved(&validate, edge)?;', """    let before = tm_project_session::host_snapshot();
    let start = std::time::Instant::now();
    let result = proved(&validate, edge);
    let after = tm_project_session::host_snapshot();
    tm_project_session::host_wall(
        tm_project_session::HostPhase::Validation,
        (start.elapsed().as_secs_f64() - (after.0 - before.0) - (after.1 - before.1)).max(0.),
    );
    result?;""")
s = s.replace('.map(|report| super::schemas::answer(&report, format))', """.map(|report| {
            let start = std::time::Instant::now();
            let answer = super::schemas::answer(&report, format);
            tm_project_session::host_wall(tm_project_session::HostPhase::Conversion, start.elapsed().as_secs_f64());
            answer
        })""")
s = s.replace('Ok(answer) => crate::output::present(format, &answer, answer.ending()),', """Ok(answer) => {
            let start = std::time::Instant::now();
            let outcome = crate::output::present(format, &answer, answer.ending());
            tm_project_session::host_wall(tm_project_session::HostPhase::Emission, start.elapsed().as_secs_f64());
            outcome
        },""")
command.write_text(s)
# The release uses separate adapters for their cache policies. A cold check
# shares one linter across all three roles. An opt-in retained run keeps the
# repository's cache separate from the fixture and counting calls.
composition = cli / "resources/workspace/check/composition.rs"
s = composition.read_text()
at = s.index("    let counting: Arc<dyn analysis::interface::AnalysisPort>")
end = s.index("    let reporting", at)
composition.write_text(s[:at] + """    let counting: Arc<dyn analysis::interface::AnalysisPort> =
        if std::env::var_os("TM_CHECK_SESSION").as_deref() == Some(std::ffi::OsStr::new("retained")) {
            Arc::new(analysis::factory::ryo(
                crate::languages::linked(),
                analysis::types::AnalysisSettings {
                    pool,
                    memory: analysis::types::Memory::Machine,
                },
            ))
        } else {
            Arc::clone(&analysis)
        };
    let scenarios = Arc::clone(&counting);
""" + s[end:])

# This module transforms the pinned benchmark host only.
import runpy
runpy.run_path(str(library / "link-rounds.py"))["link_rounds"](core, library)

# Build all three pack harnesses in this workspace so the SDK crate identity,
# linked executor and profile match the real check.
image = image_sources / "main.rs"
cli_manifest = cli.parent / "Cargo.toml"
cli_manifest.write_text(cli_manifest.read_text().replace("[dependencies]\n", "[dependencies]\ncpu-time.workspace = true\ntoml.workspace = true\nglobset = \"0.4\"\n", 1))
with cli_manifest.open("a") as manifest:
    for binary, language in (("vec", "python"), ("vec_ts", "typescript"), ("vec_rust", "rust")):
        path = cli / "bin" / (binary + ".rs")
        path.parent.mkdir(exist_ok=True)
        if language == "python":
            configure = """
    use provider::settings::{PythonParserSettings, PythonTargetVersion};
    match settings {
        None => Ok(provider::linked()),
        Some("python-target=3.10") => Ok(provider::linked_with_settings(PythonParserSettings::new(PythonTargetVersion::Py310))),
        Some(value) => Err(format!("unsupported provider settings {value}")),
    }"""
        else:
            configure = """
    match settings {
        None => Ok(provider::linked()),
        Some(value) => Err(format!("unsupported provider settings {value}")),
    }"""
        path.write_text(f"use ryo_{language} as provider;\n"
            f"const DISTINCT_LOCATIONS: bool = {str(language == 'python').lower()};\n"
            f"fn configured_provider(settings: Option<&str>) -> Result<ryo::languages::Language, String> {{{configure}\n}}\n"
            f"include!({json.dumps(str(image))});\n")
        manifest.write(f'\n[[bin]]\nname = "{binary}"\npath = "src/bin/{binary}.rs"\n')

# The linked SDK has two crate identities. Pair the concrete libraries at the
# composition root; matching a name cannot prove equivalence of parser revisions.
engine_tree = image_sources.parents[3]
sdk_manifest = sdk_src.parent / "Cargo.toml"
def dependency(path, alias, package, location, features=()):
    features = ", features = " + json.dumps(list(features)) if features else ""
    content = path.read_text()
    path.write_text(content.replace("[dependencies]\n", "[dependencies]\n" +
        f'{alias} = {{ package = "{package}", path = {json.dumps(str(location))}{features} }}\n', 1))

dependency(sdk_manifest, "native_sdk", "idiomatic-sh-ryo-sdk", engine_tree / "crates/sdk")
language_type = sdk_src / "domain/language/types.rs"
definition = language_type.read_text()
definition = definition.replace("pub struct Language {", """pub struct Language {
    pub native_provider: Option<native_sdk::languages::Language>,""", 1)
definition = definition.replace("        Self {\n            name,", "        Self {\n            native_provider: None,\n            name,", 1)
language_type.write_text(definition)
composition = cli / "languages.rs"
edges = composition.read_text()
for name in ("rust", "toml", "json", "markdown", "typescript", "python"):
    dependency(cli_manifest, "native_" + name, "idiomatic-sh-ryo-" + name, engine_tree / "languages" / name, ("fixtures",) if name == "python" else ())
    if name in ("rust", "toml", "json", "markdown"):
        factory = "linked_0_1_4" if name == "rust" else "linked"
        edges = edges.replace(f'ryo_{name}::linked(),', f'{{ let mut provider = ryo_{name}::linked(); provider.native_provider = Some(native_{name}::{factory}()); provider }},')
composition.write_text(edges)
for binary, name in (("vec", "python"), ("vec_ts", "typescript"), ("vec_rust", "rust")):
    file = cli / "bin" / (binary + ".rs")
    source = file.read_text().replace('Ok(provider::linked())',
        f'Ok({{ let mut p = provider::linked(); p.native_provider = Some(native_{name}::linked()); p }})')
    if name == "python":
        source = source.replace('Ok(provider::linked_with_settings(PythonParserSettings::new(PythonTargetVersion::Py310)))',
            '''Ok({ let mut p = provider::linked_with_settings(PythonParserSettings::new(PythonTargetVersion::Py310));
            p.native_provider = Some(native_python::linked_with_settings(native_python::settings::PythonParserSettings::new(native_python::settings::PythonTargetVersion::Py310))); p })''')
    file.write_text(source)
