"""Link the candidate in a fresh archive of the benchmark's pinned Tastemaker release."""
from pathlib import Path
import sys

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
workspace.write_text(cfg.replace(k,k+'idiomatic-sh-ryo-relations = { path = "../ryo/crates/relations" }\n'))
# Keep SDK calls on a single scheduler thread. The in-process session owns its workers.
for operation in ('check', 'validate'):
    connections = release / f'crates/tastemaker/cli/src/resources/workspace/{operation}/connections.rs'
    source = connections.read_text()
    chooser = 'std::thread::available_parallelism().map_or(1, std::num::NonZero::get)'
    assert source.count(chooser) == 1
    connections.write_text(source.replace(chooser, '1'))

adapter = core / 'src/outbound/analysis/adapters/ryo/adapter.rs'
s = adapter.read_text()
start = s.index('        let linter = self', s.index('    fn lint(&self,'))
end = s.index('\n    }', start)
s = s[:start] + '''        // Validation and the workspace check share the caller's existing worker.
        // Its thread-local state ends when this check's pool is dropped.
        let pool = self.settings.pool.0.as_ref().map_err(engine)?;
        if pool.current_num_threads() != 1 {
            return Err(engine("this benchmark requires one worker"));
        }
        thread_local! { static SESSION: std::cell::RefCell<tm_project_session::Live> = Default::default(); }
        let budget = match self.settings.memory {
            Memory::Machine => ryo::linter::MemoryBudget::Machine,
            Memory::Bytes(bytes) => std::num::NonZero::new(bytes.0).map_or(
                ryo::linter::MemoryBudget::Machine, ryo::linter::MemoryBudget::Bytes),
        };
        pool.install(|| SESSION.with(|s| {
            let mut session=s.borrow_mut();
            session.use_cache(self.cache.as_deref())?;
            session.lint(sdk,&self.languages,budget,tm_project_session::check_options())
        }))
            .map(|report| {
                let start = std::time::Instant::now();
                let output = schemas::session_report(report);
                tm_project_session::host_wall(tm_project_session::HostPhase::Conversion, start.elapsed().as_secs_f64());
                output
            })
            .map_err(engine)''' + s[end:]
adapter.write_text(s)
schema = adapter.with_name('schemas.rs')
with schema.open('a') as f:
    f.write('''
/// Render the candidate's typed findings through the same cells as a release SDK call.
pub(super) fn session_report(report: tm_project_session::LiveReport) -> AnalysisReport {
    AnalysisReport {
        findings: report.findings.into_iter().map(|(id, subject, columns)| {
            let path = subject_path(subject);
            let cells = columns.into_iter().map(|(name,value)| cell(&path,ColumnName(name),value)).collect();
            LintAnswer { rule: LintRuleId(id), path, cells }
        }).collect(),
        unknowns: report.unknowns.into_iter().map(|(id,path,msg)| LintUnknown {
            rule:LintRuleId(id), path:RepoPath(path.into()), message:MessageText(msg.into())
        }).collect(),
        unread: report.unread.into_iter().map(RepoPath).collect(),
    }
}
''')


def link_capture(release, library):
    cli = release / 'crates/tastemaker/cli'
    manifest = cli / 'Cargo.toml'
    s = manifest.read_text()
    deps = '[dependencies]\n'
    assert s.count(deps) == 1
    manifest.write_text(s.replace(deps, deps + f'tm-project-session = {{ path = "{library}" }}\n'))
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
entry.write_text(s.replace(original, '    let result = tastemaker_cli::resources::run(&start, &build.0);\n    tm_project_session::print_measurement();\n    result'))

# The benchmark replaces the released SDK constructor and reuses the CLI's reader factory.
adapter = release / 'crates/tastemaker/crates/core/src/outbound/analysis/adapters/ryo/adapter.rs'
s = adapter.read_text()
start = s.index('    /// The pool ryo reads')
s = s[:start] + '    settings: AnalysisSettings,\n    cache: Option<std::path::PathBuf>,' + s[s.index('\n}', start):]
start=s.index('        settings: AnalysisSettings,')
end=s.index('\n    }',start)
s=s[:start]+'''        mut settings: AnalysisSettings,
        cache: ryo::linter::Cache,
    ) -> Self {
        // Benchmark the real SDK budget even when the pinned CLI requests Machine.
        if let Some(value) = std::env::var_os("TM_BENCH_MEMORY_BYTES") {
            let bytes = value.to_str().and_then(|s| s.parse::<std::num::NonZeroU64>().ok())
                .expect("TM_BENCH_MEMORY_BYTES must be a positive byte count").get();
            let limit = match settings.memory {
                Memory::Machine => bytes,
                Memory::Bytes(current) => bytes.min(current.0),
            };
            settings.memory = Memory::Bytes(crate::outbound::analysis::types::MemoryBytes(limit));
        }
        Self { languages, settings, cache: match cache { ryo::linter::Cache::At(p) => Some(p), _ => None } }'''+s[end:]
s=s.replace('self, error, operator_page, placed_columns, relation_page, report','self, operator_page, placed_columns, relation_page')
adapter.write_text(s)
schema=adapter.with_name('schemas.rs')
s=schema.read_text()
for function in ('report','error'):
    k=f'pub(super) fn {function}('
    s=s.replace(k,'#[allow(dead_code, reason = "the original SDK report edge also serves pinned adapter tests")]\n'+k)
schema.write_text(s)
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
