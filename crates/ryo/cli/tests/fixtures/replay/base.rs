//! What every replay of the recorded rules shares: the recordings, the lint over a repository with the real languages and ports,
//! and the copy of ruff at its pin. Each suite that runs the recorded rules includes it whole.

#[path = "snapshot.rs"]
pub(crate) mod snapshot;

use ryo_core::inbound::use_cases::lint::interface::LintUseCase as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ryo_core::inbound::use_cases::lint::types::{LintRequest, ParameterValue, Rule, RuleParameter};
use ryo_core::inbound::use_cases::lint::use_case::LintUseCaseImpl;
use ryo_core::outbound::language::factory::LanguageConnection;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_values::domain::runs::types::Budget;
use ryo_values::domain::runs::types::Session;

use ryo_core::outbound::row_sink::interface::RowSinkPort;
use ryo_core::outbound::{corpus, language, memory, row_sink, scheduler};
use ryo_execution::outbound::{matching, query_language};
use ryo_execution_runtime::outbound::{digest, facts};
use ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl;
use ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl;
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::plan_parts::types::{
    FlagState, LiteralText, ParameterName, QueryText as CoreQueryText, WholeNumber,
};

/// The directory of the recordings.
pub(crate) fn directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/recordings")
}

/// Whether a recorded rule waits for something ryo does not do yet: only a relation its language does not provide, which the
/// recording's `pending.txt` names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Pending {
    /// The rule runs and is replayed.
    Missing,
    /// The rule waits, for this reason.
    Because(String),
}

/// What was recorded of one rule.
pub(crate) struct Recording {
    /// The rule's name, the directory it was recorded in.
    pub name: String,
    /// The rule's RyoQL query.
    pub query: String,
    /// The values the rule's parameters were given.
    pub parameters: Vec<RuleParameter>,
    /// The globs of the files the rule reads.
    pub include: Vec<String>,
    /// The globs of the files the rule leaves out.
    pub exclude: Vec<String>,
    /// Whether the rule runs and is replayed, or waits and why.
    pub pending: Pending,
}

/// The text of a recorded file, empty when the recording does not hold it.
pub(crate) fn text(path: &Path) -> String {
    match std::fs::read_to_string(path) {
        Ok(read) => read,
        // A recording holds only the files it needs: an absent one is empty.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => panic!("{}: {error}", path.display()),
    }
}

/// The text of a file a recording or a fixture must hold: an absent one fails.
pub(crate) fn required_text(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// A list of text of a table of `file`, by key: absent, it is empty; anything but a list of text fails.
pub(crate) fn texts(table: &toml::Table, key: &str) -> Vec<String> {
    let Some(value) = table.get(key) else {
        return Vec::new();
    };
    value
        .as_array()
        .unwrap_or_else(|| panic!("`{key}` is not a list: {value}"))
        .iter()
        .map(|each| {
            each.as_str()
                .unwrap_or_else(|| panic!("`{key}` holds `{each}`, which is not text"))
                .to_owned()
        })
        .collect()
}

/// What a parameter of the recording is bound to, when ryo can bind it.
pub(crate) fn parameter(name: &str, value: &toml::Value) -> Option<RuleParameter> {
    let bound = match value {
        toml::Value::Integer(number) => ParameterValue::Number(WholeNumber(*number)),
        toml::Value::Boolean(flag) => ParameterValue::Flag(if *flag {
            FlagState::Raised
        } else {
            FlagState::Lowered
        }),
        toml::Value::String(each) => ParameterValue::Text(LiteralText(each.as_str().into())),
        toml::Value::Array(items) if items.iter().all(toml::Value::is_str) => {
            ParameterValue::Texts(
                items
                    .iter()
                    .filter_map(|each| each.as_str().map(|text| LiteralText(text.into())))
                    .collect(),
            )
        }
        toml::Value::Array(items) if items.iter().all(toml::Value::is_integer) => {
            ParameterValue::Numbers(
                items
                    .iter()
                    .filter_map(toml::Value::as_integer)
                    .map(WholeNumber)
                    .collect(),
            )
        }
        // A list of sets of text (`list<set<string>>`), each set an array of text.
        toml::Value::Array(items)
            if items.iter().all(|set| {
                set.as_array()
                    .is_some_and(|texts| texts.iter().all(toml::Value::is_str))
            }) =>
        {
            ParameterValue::TextSets(
                items
                    .iter()
                    .filter_map(toml::Value::as_array)
                    .map(|set| {
                        set.iter()
                            .filter_map(|each| each.as_str().map(|text| LiteralText(text.into())))
                            .collect()
                    })
                    .collect(),
            )
        }
        toml::Value::Float(_)
        | toml::Value::Datetime(_)
        | toml::Value::Array(_)
        | toml::Value::Table(_) => return None,
    };
    Some(RuleParameter {
        name: ParameterName(name.into()),
        value: bound,
    })
}

/// The plan ryo compiles a query to, or why it does not compile it.
pub(crate) fn compiled(
    query: &str,
) -> Result<
    ryo_values::domain::plans::types::Plan,
    ryo_query_language::inbound::use_cases::compile::errors::CompileError,
> {
    let compiler = CompileUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    );
    ryo_query_language::inbound::use_cases::compile::interface::CompileUseCase::compile(
        &compiler,
        ryo_values::domain::plans::types::QueryRequest {
            observations: Vec::new(),
            text: ryo_values::domain::query_tokens::types::QueryBundle::from(CoreQueryText(
                query.into(),
            )),
            hosted: Vec::new(),
            modules: ryo_values::domain::program::types::ParsedModules::default(),
        },
    )
    .map(|report| report.plan)
}

/// The refusal under every module origin a compile refusal is located at: a query reading through a module is refused `Located`.
fn innermost(
    error: &ryo_query_language::inbound::use_cases::compile::errors::CompileError,
) -> &ryo_query_language::inbound::use_cases::compile::errors::CompileError {
    match error {
        ryo_query_language::inbound::use_cases::compile::errors::CompileError::Located {
            error,
            ..
        } => innermost(error),
        other => other,
    }
}

/// The names of the relations of ryo a plan reads, each once, in order.
pub(crate) fn relations_read(plan: &ryo_values::domain::plans::types::Plan) -> Vec<String> {
    let mut read = std::collections::BTreeSet::new();
    for step in &plan.steps {
        let mut operation = step.operation.clone();
        ryo_values::domain::plans::service::each_input(&mut operation, &mut |input| {
            if let ryo_values::domain::plan_parts::types::Input::Relation(relation) = input {
                let _new = read.insert(
                    ryo_relations::domain::relations::service::name(*relation)
                        .0
                        .to_owned(),
                );
            }
        });
    }
    read.into_iter().collect()
}

/// Every recorded rule, in the order of their names.
pub(crate) fn recordings() -> Vec<Recording> {
    let mut found: Vec<Recording> = std::fs::read_dir(directory())
        .expect("a folder of the fixtures is read")
        .map(|entry| entry.expect("an entry of a folder of the fixtures is read"))
        .filter(|entry| entry.path().is_dir())
        .map(|entry| recording_at(&entry.path()))
        .collect();
    found.sort_by(|left, right| left.name.cmp(&right.name));
    found
}

/// A recorded table of `file` under `at`: an absent file is an empty table, a malformed one fails.
pub(crate) fn table_at(at: &Path, file: &str) -> toml::Table {
    text(&at.join(file))
        .parse()
        .unwrap_or_else(|error| panic!("{}: {error}", at.join(file).display()))
}

/// The recording in the directory `at`, checked. It must hold its query and its scope. A rule that does not compile, binds a
/// parameter ryo cannot bind, leaves a parameter its query declares without a value, reads a relation the Rust language does not
/// provide, or is refused by the engine fails. The one exception is a `pending.txt`, which names exactly the relations the rule
/// reads that its language does not provide: the query must fail on no other ground, and either name one relation ryo does not
/// own (the compile refusal `no relation named`) or compile to a plan whose unprovided relations are exactly those named.
pub(crate) fn recording_at(at: &Path) -> Recording {
    let name = at
        .file_name()
        .map(|each| each.to_string_lossy().into_owned())
        .unwrap_or_default();
    let query = required_text(&at.join("query.ryoql"));
    let _scope_held = required_text(&at.join("scope.toml"));
    let scope = table_at(at, "scope.toml");
    // A scope says which files the rule was recorded over: its `include` globs, and the `exclude` globs it may add. Nothing else.
    assert!(
        scope.contains_key("include")
            && scope.keys().all(|key| key == "include" || key == "exclude"),
        "{}: scope.toml holds {:?}, not `include` and an optional `exclude`",
        at.display(),
        scope.keys().collect::<Vec<_>>()
    );
    let given = table_at(at, "parameters.toml");
    let unbound: Vec<&String> = given
        .iter()
        .filter(|(name, value)| parameter(name, value).is_none())
        .map(|(name, _)| name)
        .collect();
    let plan = compiled(&query);
    let undeclared: Vec<String> = plan
        .iter()
        .flat_map(|plan| plan.parameters.iter())
        .filter(|declared| !given.contains_key(&*declared.name.0))
        .map(|declared| declared.name.0.to_string())
        .collect();
    let pending =
        Some(text(&at.join("pending.txt")).trim().to_owned()).filter(|why| !why.is_empty());
    let mut recording = Recording {
        name,
        query,
        parameters: given
            .iter()
            .filter_map(|(name, value)| parameter(name, value))
            .collect(),
        include: texts(&scope, "include"),
        exclude: texts(&scope, "exclude"),
        pending: Pending::Missing,
    };
    let name = recording.name.clone();
    let named = pending.as_deref().map(unprovided);
    match (plan.as_ref().map_err(innermost), &named) {
        (
            Err(ryo_query_language::inbound::use_cases::compile::errors::CompileError::UnknownRelation { name: unknown, .. }),
            Some(named),
        ) => {
            assert!(
                named.as_slice() == std::slice::from_ref(unknown),
                "{name}: the query reads `{unknown}`, which ryo does not own, and pending.txt names {named:?}"
            );
        }
        (Err(error), _) => panic!("{name}: ryo does not compile the recorded query: {error}"),
        (Ok(plan), _) => {
            let mut absent: Vec<String> = relations_read(plan)
                .into_iter()
                .filter(|relation| !provided(relation))
                .collect();
            absent.sort();
            let mut expected = named.clone().unwrap_or_default();
            expected.sort();
            assert!(
                absent == expected,
                "{name}: the plan reads {absent:?}, which the Rust language does not provide, and pending.txt names {expected:?}"
            );
            assert!(unbound.is_empty(), "{name}: ryo cannot bind the parameters {unbound:?}");
            assert!(
                undeclared.is_empty(),
                "{name}: the query declares {undeclared:?}, which parameters.toml gives no value"
            );
            let stray: Vec<&String> = given
                .keys()
                .filter(|given| !plan.parameters.iter().any(|declared| &*declared.name.0 == given.as_str()))
                .collect();
            assert!(stray.is_empty(), "{name}: parameters.toml gives {stray:?}, which the query does not declare");
            if let Some(refusal) = does_not_run(&recording) {
                panic!("{name}: {refusal}");
            }
        }
    }
    if let Some(why) = pending {
        recording.pending = Pending::Because(why);
    }
    recording
}

/// The relations a `pending.txt` names, in its one form: `reads <relation>, <relation>, which the Rust language of ryo does not
/// provide`.
pub(crate) fn unprovided(why: &str) -> Vec<String> {
    let named = why
        .strip_prefix("reads ")
        .and_then(|rest| rest.split_once(", which the Rust language of ryo does not provide"))
        .map(|(names, _)| names)
        .unwrap_or_else(|| panic!("pending.txt says `{why}`, not `reads <relations>, which the Rust language of ryo does not provide`"));
    let relations: Vec<String> = named.split(", ").map(str::to_owned).collect();
    assert!(
        relations.iter().all(|relation| !relation.is_empty()),
        "pending.txt names an empty relation: `{why}`"
    );
    relations
}

/// Whether the Rust language, the one the replay links, provides the relation RyoQL calls `relation`: a relation ryo does not
/// own (a host's) is provided by no language.
pub(crate) fn provided(relation: &str) -> bool {
    use ryo_core::outbound::language::interface::LanguagePort as _;
    let Some(owned) = ryo_relations::domain::relations::service::relation_named(
        ryo_relations::domain::labels::types::NameRef(relation),
    ) else {
        return false;
    };
    language::factory::use_case(vec![LanguageConnection::from(ryo_rust::linked())])
        .supports(owned)
        .0
        .iter()
        .any(|answer| answer.absence == language::types::Absence::Reads)
}

/// The real store, kept in the cache directory under `root`, for the build the tests are.
pub(crate) fn store_at(
    root: &Path,
) -> Arc<dyn ryo_core::outbound::lint_store::interface::LintStorePort> {
    Arc::new(ryo_core::outbound::lint_store::factory::postcard(
        ryo_core::outbound::lint_store::types::Store {
            directory: ryo_core::outbound::lint_store::types::CacheDirectory(
                root.join(".ryo_cache").to_string_lossy().into(),
            ),
            build: ryo_core::outbound::lint_store::types::BuildIdentity("replay".into()),
        },
    ))
}

/// Lints the files under `root` with `rules`, with the real engine: every row of every finding, and the files nothing read.
pub(crate) fn lint_over(
    root: &Path,
    rules: Vec<Rule>,
) -> Result<(Vec<Vec<String>>, Vec<String>), String> {
    let spill = tempfile::tempdir().map_err(|error| error.to_string())?;
    lint_with(
        root,
        rules,
        Arc::new(facts::factory::spill(spill.path().to_path_buf())),
        Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
    )
}

/// Whether the lint counts the rows each step is fed, which the replay of ruff sets for the report of features.
pub(crate) static COUNTING: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Lints `root` with `rules`, the facts kept by `store`: the rows the rules gave and the files nothing read.
///
/// # Errors
///
/// What the lint refuses, as text.
pub(crate) fn lint_with(
    root: &Path,
    rules: Vec<Rule>,
    store: Arc<dyn ryo_execution_runtime::outbound::facts::interface::FactsPort>,
    lint_store: Arc<dyn ryo_core::outbound::lint_store::interface::LintStorePort>,
) -> Result<(Vec<Vec<String>>, Vec<String>), String> {
    let (rows, report) = lint_report(root, rules, store, lint_store)?;
    Ok((
        rows,
        report
            .unread
            .into_iter()
            .map(|file| file.path.0.to_string())
            .collect(),
    ))
}

/// Lints `root` with `rules`, the facts kept by `store`: the rows the rules gave and the report of the lint.
///
/// # Errors
///
/// What the lint refuses, as text.
pub(crate) fn lint_report(
    root: &Path,
    rules: Vec<Rule>,
    store: Arc<dyn ryo_execution_runtime::outbound::facts::interface::FactsPort>,
    lint_store: Arc<dyn ryo_core::outbound::lint_store::interface::LintStorePort>,
) -> Result<
    (
        Vec<Vec<String>>,
        ryo_core::inbound::use_cases::lint::types::LintReport,
    ),
    String,
> {
    // The command line links the TOML language beside the Rust language; a rule that reads a manifest's tree needs it.
    lint_report_hosted(root, rules, store, lint_store, Vec::new(), true)
}

/// Lints `root` with `rules`, the facts kept by `store`, over the relations a host declares beside ryo's own: the rows the rules gave and
/// the report of the lint.
///
/// # Errors
///
/// What the lint refuses, as text.
pub(crate) fn lint_report_hosted(
    root: &Path,
    rules: Vec<Rule>,
    store: Arc<dyn ryo_execution_runtime::outbound::facts::interface::FactsPort>,
    lint_store: Arc<dyn ryo_core::outbound::lint_store::interface::LintStorePort>,
    hosted: Vec<ryo_values::domain::hosted::types::HostRelation>,
    manifests_read: bool,
) -> Result<
    (
        Vec<Vec<String>>,
        ryo_core::inbound::use_cases::lint::types::LintReport,
    ),
    String,
> {
    lint_report_with_languages(
        root,
        rules,
        store,
        lint_store,
        hosted,
        manifests_read,
        Vec::new(),
    )
}

/// The same production composition with additional providers explicitly selected by a scene.
///
/// # Errors
///
/// What the lint refuses, as text.
pub(crate) fn lint_report_with_languages(
    root: &Path,
    rules: Vec<Rule>,
    store: Arc<dyn ryo_execution_runtime::outbound::facts::interface::FactsPort>,
    lint_store: Arc<dyn ryo_core::outbound::lint_store::interface::LintStorePort>,
    hosted: Vec<ryo_values::domain::hosted::types::HostRelation>,
    manifests_read: bool,
    additional: Vec<LanguageConnection>,
) -> Result<
    (
        Vec<Vec<String>>,
        ryo_core::inbound::use_cases::lint::types::LintReport,
    ),
    String,
> {
    let rust = ryo_rust::linked();
    let mut handles = vec![LanguageConnection::from(rust)];
    handles.extend(additional);
    // Rules that locate a finding in a manifest read its tree, which only the TOML language gives.
    if manifests_read {
        let toml = ryo_toml::linked();
        handles.push(LanguageConnection::from(toml));
    }
    let compile = CompileUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    );
    let collected = row_sink::factory::CollectorConnection::default();
    let kept: Arc<dyn RowSinkPort> = Arc::new(row_sink::factory::collect(collected.clone()));
    let use_case = {
        let analysis_corpus: Arc<dyn corpus::interface::CorpusPort> =
            Arc::new(corpus::factory::ignore(
                RootPath(root.to_string_lossy().into()),
                ryo_values::domain::files::types::CorpusListing::Walked,
                Vec::new(),
                corpus::types::SkippedDirectories(vec![".ryo_cache".into()]),
            ));
        let analysis_language: Arc<dyn language::interface::LanguagePort> =
            Arc::new(language::factory::use_case(handles));
        let analysis_memory: Arc<dyn memory::interface::MemoryPort> =
            Arc::new(memory::factory::sysinfo(Budget::Machine));
        let analysis_digest: Arc<
            dyn ryo_execution_runtime::outbound::digest::interface::DigestPort,
        > = Arc::new(digest::factory::blake3());
        let started = scheduler::types::WorkerCount(
            ryo_values::domain::runs::service::workers_to_start(
                ryo_values::domain::runs::types::WorkersAsked(0),
                ryo_values::domain::runs::types::MachineWorkers(
                    u32::try_from(
                        std::thread::available_parallelism().map_or(1, std::num::NonZero::get),
                    )
                    .unwrap_or(1),
                ),
            )
            .0,
        );
        LintUseCaseImpl::new(
            ryo_core::application::file_analysis::service::FileAnalysis::new(
                Arc::clone(&analysis_corpus),
                Arc::clone(&analysis_language),
                Arc::clone(&analysis_digest),
                Arc::clone(&analysis_memory),
            ),
            ryo_core::application::cross_file::service::CrossFile::new(Arc::clone(
                &analysis_corpus,
            ), std::sync::Arc::new(ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl::new())),
            Arc::clone(&analysis_corpus),
            store,
            Arc::clone(&analysis_language),
            Arc::new(matching::factory::globset()),
            Arc::new(query_language::factory::use_case(
                Arc::new(compile),
                Arc::new(TypecheckUseCaseImpl::new(
                    Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
                    Arc::new(ryo_query_language::outbound::catalog::factory::language()),
                )),
            )),
            Arc::clone(&analysis_memory),
            Arc::new(ryo_execution_runtime::outbound::operators::factory::use_case(
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())),
            )),
            ryo_execution::derived(),
            Arc::new(scheduler::factory::rayon(started)),
            started,
            Arc::clone(&kept),
            Arc::clone(&analysis_digest),
            lint_store,
        )
    };
    let use_case = if COUNTING.load(std::sync::atomic::Ordering::Relaxed) {
        use_case.counting()
    } else {
        use_case
    };
    let request = LintRequest {
        source_language: ryo_core::inbound::use_cases::lint::types::SourceProvider::Recognized,
        answer: ryo_values::domain::runs::types::AnswerShape::Findings,
        hosted,
        session: Session {
            search_scope: Default::default(),
            given_inventory: Default::default(),
            repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
            root: RootPath(root.to_string_lossy().into()),
        },
        files: Vec::new(),
        rules,
    };
    let report = use_case
        .lint(request)
        .map_err(|error| format!("the recorded rules do not run: {error}"))?;
    let rows = base_rows(&collected);
    Ok((rows, report))
}

/// The rule of a recording, as ryo is asked to run it.
pub(crate) fn rule_of(each: &Recording) -> Rule {
    Rule {
        observations: Vec::new(),
        id: ryo_values::domain::runs::types::RuleId(each.name.as_str().into()),
        query: ryo_values::domain::query_tokens::types::QueryBundle::from(CoreQueryText(
            each.query.as_str().into(),
        )),
        applicability: ryo_values::domain::runs::types::RuleApplicability::Always,
        parameters: each.parameters.clone(),
        include: each
            .include
            .iter()
            .map(|glob| Glob(glob.as_str().into()))
            .collect(),
        exclude: each
            .exclude
            .iter()
            .map(|glob| Glob(glob.as_str().into()))
            .collect(),
    }
}

/// Why a rule does not run, found on a few lines of Rust: the engine refuses it at run time, for a table it builds from what the query asks.
pub(crate) fn does_not_run(each: &Recording) -> Option<String> {
    does_not_run_in(tempfile::tempdir(), each)
}

/// Why a rule does not run in the sample directory `sample`: a sample that could not be made or written is a failure too, never a
/// rule that runs.
pub(crate) fn does_not_run_in(
    sample: std::io::Result<tempfile::TempDir>,
    each: &Recording,
) -> Option<String> {
    let sample = match sample {
        Ok(sample) => sample,
        Err(error) => {
            return Some(format!(
                "the sample to run the recorded query in was not made: {error}"
            ));
        }
    };
    if let Err(error) = std::fs::write(
        sample.path().join("lib.rs"),
        "use std::fmt;\nfn run(value: u8) -> u8 { value }\nstruct Point { x: u8 }\nimpl Point { fn new() -> Self { Point { x: 0 } } }\n",
    ) {
        return Some(format!(
            "the sample to run the recorded query in was not written: {error}"
        ));
    }
    lint_over(sample.path(), vec![rule_of(each)])
        .err()
        .map(|failure| format!("ryo does not run the recorded query: {failure}"))
}

/// The findings a lint kept, each as the text a recording holds: the rule, the subject, the key, the line and the character.
pub(crate) fn base_rows(collected: &row_sink::factory::CollectorConnection) -> Vec<Vec<String>> {
    std::mem::take(
        &mut *collected
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    )
    .rows
    .iter()
    .map(|row| finding_texts(&row.0))
    .collect()
}

/// How a cell a row does not have reads, apart from a cell of empty text.
pub(crate) const ABSENT: &str = "(absent)";

/// A finding's typed row as the text a recording holds: its rule, its subject, its key values joined with commas, its line and its
/// character, each value as it reads (a span as its two ends); its columns, which a recording does not hold, left out. A row not laid
/// out as a finding's is each of its cells as text.
pub(crate) fn finding_texts(
    cells: &[Option<ryo_values::domain::row_texts::types::Cell>],
) -> Vec<String> {
    use ryo_values::domain::row_texts::types::Cell;
    let text = |cell: &Option<Cell>| match cell {
        Some(Cell::File(each) | Cell::Text(each)) => each.to_string(),
        Some(Cell::Number(number)) => number.0.to_string(),
        Some(Cell::Span(span))
            if ryo_relations::domain::relations::service::location(*span)
                == ryo_relations::domain::relations::types::Locatedness::Absent =>
        {
            ryo_relations::domain::relations::constants::UNLOCATED.to_owned()
        }
        Some(Cell::Span(span)) => format!("{}..{}", span.start.0, span.end.0),
        None => ABSENT.to_owned(),
    };
    match cells {
        [
            rule,
            subject,
            Some(Cell::Number(count)),
            rest @ ..,
            line,
            column,
        ] => {
            let Ok(count) = usize::try_from(count.0) else {
                return cells.iter().map(text).collect();
            };
            let key: Vec<String> = rest.iter().take(count).map(text).collect();
            vec![
                text(rule),
                text(subject),
                key.join(","),
                text(line),
                text(column),
            ]
        }
        _ => cells.iter().map(text).collect(),
    }
}
