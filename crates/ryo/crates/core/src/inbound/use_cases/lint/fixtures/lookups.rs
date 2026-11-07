//! The cases of a project lookup between two runs, of compiled plans one session shares, and of files read again.
use super::super::interface::LintUseCase as _;
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::lint_store::fixtures::fake::LintStoreFake;
use crate::outbound::scheduler::fixtures::fake::SchedulerFake;
use crate::outbound::scheduler::interface::SchedulerPort;
use crate::outbound::scheduler::types::WorkerCount;
use ryo_language::domain::files::types::FilePath;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use super::given::Given;
use super::given::SOURCES;
use super::given::World;
use super::given::request;
use super::given::violations;
use super::lifecycle::counted_lint;
use super::lifecycle::reading_the_project;
use super::super::errors::LintError;
use super::worlds::again;
use super::worlds::scheduled_world_with_query;
use super::worlds::world;

/// What changes between two lints of a rule that reads the project, whose lookup reads a configuration above the root.
#[derive(Clone, Copy)]
pub(crate) enum Between {
    /// Nothing.
    Nothing,
    /// The configuration above the root.
    ConfigurationAbove,
    /// The manifest the run lists.
    Manifest,
    /// The configuration above the root is gone.
    ConfigurationGone,
    /// A configuration the lookup looked for above the root and found none of is there now.
    ConfigurationAppears,
    /// The lookup found the older configuration above the root, and the newer one that comes before it is there now.
    OlderBecomesNewer,
}

/// Lints a rule that reads the project, over a manifest the run lists and a configuration above the root the language's lookup
/// reads, changes what `between` says, and lints again over the same store: answers how many times the second lint read the project,
/// and 1 when the steps of its rule were fed rows, 0 when it made nothing again.
pub(crate) fn consulted(between: Between) -> Result<(usize, usize), LintError> {
    let store = Arc::new(LintStoreFake::default());
    let manifest = ("project.manifest.fake", "members = 1");
    let configuration = ("../config.fake", "target = 1");
    let older = ("../older.fake", "target = 0");
    let given = |listed: (&'static str, &'static str),
                 outside: Vec<(&'static str, &'static str)>| Given {
        files: vec![SOURCES[0], SOURCES[1], listed],
        outside,
        store: Arc::clone(&store),
        root: "root",
        query: Arc::new(reading_the_project()),
        language: LanguageFake {
            // The lookup asks for the newer configuration first, then the older one.
            consulted: vec![FilePath(configuration.0.into()), FilePath(older.0.into())],
            ..LanguageFake::default()
        },
        ..violations()
    };
    let first = match between {
        Between::ConfigurationAppears => Vec::new(),
        Between::OlderBecomesNewer => vec![older],
        Between::Nothing
        | Between::ConfigurationAbove
        | Between::Manifest
        | Between::ConfigurationGone => vec![configuration],
    };
    let (read, fed) = counted_lint(given(manifest, first))?;
    assert_eq!(read, 1, "a cold run reads the project");
    assert!(fed > 0, "and feeds the steps of its rule");
    let (manifest, outside) = match between {
        Between::Nothing | Between::ConfigurationAppears => (manifest, vec![configuration]),
        Between::ConfigurationAbove => (manifest, vec![("../config.fake", "target = 2")]),
        Between::Manifest => (
            ("project.manifest.fake", "members = 2"),
            vec![configuration],
        ),
        Between::ConfigurationGone => (manifest, Vec::new()),
        Between::OlderBecomesNewer => (manifest, vec![configuration, older]),
    };
    let (read_again, fed_again) = counted_lint(given(manifest, outside))?;
    Ok((read_again, usize::from(fed_again > 0)))
}

/// A file that cannot be read stays unread, for the same reason, on the warm run.
pub(crate) fn unreadable_again() -> Result<(usize, usize), LintError> {
    let store = Arc::new(LintStoreFake::default());
    let files = [SOURCES[0], ("unstable.rs", "fn main() {}")];
    let cold = again(&files, "root", "a rule", &store);
    let warm = again(&files, "root", "a rule", &store);
    assert_eq!(
        warm.report, cold.report,
        "the file is unread for the same reason"
    );
    warm.report
        .map(|unread| (usize::from(warm.reads > 0), unread))
}

/// Two lints of one session: answers how many times the rules were compiled, and what the second left unread.
pub(crate) fn compiles_per_session() -> Result<(usize, usize), LintError> {
    let given = Given {
        files: vec![SOURCES[0]],
        rules: vec!["one", "two"],
        ..violations()
    };
    let (first, second) = (request(&given), request(&given));
    let world = world(given);
    let warm = world.use_case.lint(first)?;
    let report = world.use_case.lint(second)?;
    assert_eq!(
        warm.unread.len(),
        report.unread.len(),
        "a second lint leaves unread what the first did"
    );
    Ok((
        world.query.calls.load(Ordering::SeqCst),
        report.unread.len(),
    ))
}

/// The same root and outer module use a new nested-module body: both caches must observe it.
pub(crate) fn changed_nested_module() -> Result<(usize, usize), LintError> {
    let bundle = |revision: &str| {
        use ryo_values::domain::program::service::{document_source, query_bundle, source_id};
        let projection = if revision == "first" {
            "id"
        } else {
            "id as \"id\""
        };
        query_bundle(
            document_source(
                source_id(ryo_values::domain::query_tokens::types::QuerySpelling("root")).unwrap(),
                "import outer from \"outer\"; let found = outer(); emit found subject id key(id);",
            ),
            [
                document_source(
                    source_id(ryo_values::domain::query_tokens::types::QuerySpelling("outer")).unwrap(),
                    "import nested from \"nested\"; output relation<id: node>; let found = nested(); return found;",
                ),
                document_source(
                    source_id(ryo_values::domain::query_tokens::types::QuerySpelling("nested")).unwrap(),
                    format!("output relation<id: node>; let found = select {projection} from \"syntax-tree\"; return found;").into_boxed_str(),
                ),
            ],
        )
        .unwrap()
    };
    let given = violations();
    let mut first = request(&given);
    first.rules[0].query = bundle("first");
    let mut second = request(&given);
    second.rules[0].query = bundle("second");
    let world = world(given);
    let _first_report = world.use_case.lint(first)?;
    let after_first = world.language.parses.load(Ordering::SeqCst);
    let _second_report = world.use_case.lint(second)?;
    let parsed_again = world.language.parses.load(Ordering::SeqCst) > after_first;
    let compiled_twice = world.query.calls.load(Ordering::SeqCst) == 2;
    Ok((usize::from(parsed_again && compiled_twice), 0))
}

/// A module-only predicate edit changes actual findings through a retained real compiler plan.
pub(crate) fn changed_fragment_plan() -> Result<(usize, usize), LintError> {
    use ryo_execution::outbound::query_language::factory;
    use ryo_execution::inbound::use_cases::prepare::interface::CompiledPlans;
    use ryo_execution::outbound::query_language::interface::QueryLanguagePort;

    let make_bundle = |prefix: &str| {
        let module = format!(
            "input files: relation<path: file>;\noutput relation<path: file>;\nlet selected = select path from files where starts_with(path, \"{prefix}\");\nreturn selected;"
        );
        ryo_values::domain::program::service::query_bundle(
            ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id(ryo_values::domain::query_tokens::types::QuerySpelling("root")).unwrap(),
                "import choose from \"query/filter\"; let files = select path from \"filesystem.file\"; let result = choose(files=files); emit result subject \"path\" key(\"path\");",
            ),
            [ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id(ryo_values::domain::query_tokens::types::QuerySpelling("query/filter")).unwrap(),
                module.into_boxed_str(),
            )],
        )
        .unwrap()
    };
    let given = Given {
        files: vec![("alpha.txt", "alpha"), ("beta.txt", "beta")],
        ..violations()
    };
    let mut first = request(&given);
    first.rules[0].query = make_bundle("alpha");
    let mut second = request(&given);
    second.rules[0].query = make_bundle("beta");
    let syntax = || Arc::new(ryo_query_language::outbound::syntax::factory::winnow());
    let catalog = || Arc::new(ryo_query_language::outbound::catalog::factory::language());
    let compile = Arc::new(
        ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl::new(
            syntax(),
            catalog(),
        ),
    );
    let typecheck = Arc::new(
        ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl::new(
            syntax(),
            catalog(),
        ),
    );
    let query_language: Arc<dyn QueryLanguagePort> =
        Arc::new(factory::use_case(compile, typecheck));
    let scheduler: Arc<dyn SchedulerPort> = Arc::new(SchedulerFake::default());
    let mut world = scheduled_world_with_query(given, scheduler, WorkerCount(1), query_language);
    let plans = CompiledPlans::default();
    world.use_case = world.use_case.compiling_into(plans.clone());

    let _first_report = world.use_case.lint(first)?;
    let first_paths = collected_file_paths(&world);
    let _second_report = world.use_case.lint(second.clone())?;
    let second_paths = collected_file_paths(&world);
    let _retained_report = world.use_case.lint(second)?;
    let retained_paths = collected_file_paths(&world);
    Ok((
        usize::from(
            first_paths == std::collections::BTreeSet::from(["alpha.txt".to_owned()])
                && second_paths == std::collections::BTreeSet::from(["beta.txt".to_owned()])
                && retained_paths == second_paths,
        ),
        0,
    ))
}

/// Takes the file subjects written by one lint run and clears the shared collector.
pub(crate) fn collected_file_paths(world: &World) -> std::collections::BTreeSet<String> {
    let rows = std::mem::take(
        &mut world
            .collected
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .rows,
    );
    rows.iter()
        .flat_map(|row| row.0.iter())
        .filter_map(|cell| match cell {
            Some(ryo_values::domain::row_texts::types::Cell::File(path)) => Some(path.to_string()),
            _ => None,
        })
        .collect()
}

/// Lints rule `a`, then rule `b`, then each again, over plans one session shares: answers how many times a rule was compiled.
pub(crate) fn interleaved_runs() -> Result<(usize, usize), LintError> {
    let rule = |id| Given {
        files: vec![SOURCES[0]],
        rules: vec!["one"],
        ids: vec![id],
        ..violations()
    };
    let world = world(rule("a"));
    for id in ["a", "b", "a", "b"] {
        let _run = world.use_case.lint(request(&rule(id)))?;
    }
    Ok((world.query.calls.load(Ordering::SeqCst), 0))
}
