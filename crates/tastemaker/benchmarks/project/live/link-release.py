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
adapter = core / 'src/outbound/analysis/adapters/ryo/adapter.rs'
s = adapter.read_text()
start = s.index('        let linter = self', s.index('    fn lint(&self,'))
end = s.index('\n    }', start)
s = s[:start] + '''        // Validation and the workspace check share the caller's existing worker.
        // Its thread-local state ends when this check's pool is dropped.
        let pool = self.pool.0.as_ref().map_err(engine)?;
        if pool.current_num_threads() != 1 {
            return Err(engine("this benchmark requires one worker"));
        }
        thread_local! { static SESSION: std::cell::RefCell<tm_project_session::Live> = Default::default(); }
        pool.install(|| SESSION.with(|s| s.borrow_mut().lint(sdk, &self.languages)))
            .map(schemas::session_report)
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
    if mode.as_deref() == Some("--tm-batch") {
        let manifest = arguments.next().expect("manifest");
        let output = arguments.next().expect("output");
        let workers = arguments.next().map(|flag| {
            assert_eq!(flag, "--workers");
            arguments.next().expect("worker count").parse().expect("positive worker count")
        });
        assert!(arguments.next().is_none(), "unexpected batch argument");
        tm_project_session::capture(benchmark_languages::linked(), &manifest, &output, workers);
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
s = s[:start] + '    pool: crate::outbound::scheduler::types::WorkerPool,' + s[s.index('\n}', start):]
start=s.index('        settings: AnalysisSettings,')
end=s.index('\n    }',start)
s=s[:start]+'''        settings: AnalysisSettings,
        _cache: ryo::linter::Cache,
    ) -> Self {
        Self { languages, pool: settings.pool }'''+s[end:]
s=s.replace('self, error, operator_page, placed_columns, relation_page, report','self, operator_page, placed_columns, relation_page').replace('InvalidReason, Memory, QueryVerdict', 'InvalidReason, QueryVerdict')
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
