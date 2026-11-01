//! What every replay of the recorded rules shares: the recordings, the lint over a repository with the real languages and ports,
//! and the copy of ruff at its pin. Each suite that runs the recorded rules includes it whole.

#[path = "snapshot.rs"]
pub(crate) mod snapshot;

use ryo_core::inbound::use_cases::lint::interface::LintUseCase as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ryo_core::inbound::use_cases::lint::types::{LintRequest, ParameterValue, Rule, RuleParameter};
use ryo_core::inbound::use_cases::lint::use_case::LintUseCaseImpl;
use ryo_core::outbound::language::types::LanguageHandles;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_values::domain::runs::types::Budget;
use ryo_values::domain::runs::types::Session;

use ryo_core::outbound::rows::interface::RowsPort;
use ryo_core::outbound::{
    corpus, digest, facts, language, matching, memory, query_language, rows, scheduler,
};
use ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl;
use ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl;
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::plan_parts::types::{
    FlagState, LiteralText, ParameterName, QueryText as CoreQueryText, WholeNumber,
};
use ryo_values::domain::plans::types::{ParameterType, PlanParameter};

/// The directory of the recordings.
pub(crate) fn directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/recordings")
}

/// Whether a recorded rule waits for something ryo does not do yet.
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

/// A list of text of a table of `file`, by key.
pub(crate) fn texts(table: &toml::Table, key: &str) -> Vec<String> {
    table
        .get(key)
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|each| each.as_str().map(str::to_owned))
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

/// The parameters a query declares, as the query language reads them; or why ryo does not compile the query.
pub(crate) fn compile(query: &str) -> Result<Vec<PlanParameter>, String> {
    let compiler = CompileUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    );
    ryo_query_language::inbound::use_cases::compile::interface::CompileUseCase::compile(
        &compiler,
        ryo_query_language::inbound::use_cases::compile::types::CompileRequest {
            text: ryo_values::domain::program::types::QueryBundle::from(CoreQueryText(
                query.into(),
            )),
            hosted: Vec::new(),
        },
    )
    .map(|report| report.plan.parameters)
    .map_err(|error| format!("ryo does not compile the recorded query: {error}"))
}

/// Every recorded rule, in the order of their names.
pub(crate) fn recordings() -> Vec<Recording> {
    let mut found: Vec<Recording> = std::fs::read_dir(directory())
        .expect("a folder of the fixtures is read")
        .map(|entry| entry.expect("an entry of a folder of the fixtures is read"))
        .filter(|entry| entry.path().is_dir())
        .map(|entry| {
            let at = entry.path();
            let scope: toml::Table = text(&at.join("scope.toml")).parse().unwrap_or_default();
            let given: toml::Table = text(&at.join("parameters.toml"))
                .parse()
                .unwrap_or_default();
            let unbound = given
                .iter()
                .any(|(name, value)| parameter(name, value).is_none());
            let query = text(&at.join("query.ryoql"));
            let compiled = compile(&query);
            let pending = Some(text(&at.join("pending.txt")).trim().to_owned())
                .filter(|why| !why.is_empty())
                .or_else(|| unbound.then(|| "a parameter ryo cannot declare or bind".to_owned()))
                .or_else(|| compiled.as_ref().err().cloned());
            let mut recording = Recording {
                name: entry.file_name().to_string_lossy().into_owned(),
                query,
                parameters: given
                    .iter()
                    .filter_map(|(name, value)| parameter(name, value))
                    .chain(
                        compiled
                            .iter()
                            .flatten()
                            .filter(|declared| !given.contains_key(&*declared.name.0))
                            .filter_map(|declared| {
                                Some(RuleParameter {
                                    name: declared.name.clone(),
                                    value: match declared.kind {
                                        ParameterType::Texts => ParameterValue::Texts(Vec::new()),
                                        ParameterType::Numbers => {
                                            ParameterValue::Numbers(Vec::new())
                                        }
                                        ParameterType::Flag
                                        | ParameterType::Number
                                        | ParameterType::Text
                                        | ParameterType::TextSets
                                        | ParameterType::Tables => return None,
                                    },
                                })
                            }),
                    )
                    .collect(),
                include: texts(&scope, "include"),
                exclude: texts(&scope, "exclude"),
                pending: pending.map_or(Pending::Missing, Pending::Because),
            };
            if recording.pending == Pending::Missing {
                recording.pending =
                    does_not_run(&recording).map_or(Pending::Missing, Pending::Because);
            }
            recording
        })
        .collect();
    found.sort_by(|left, right| left.name.cmp(&right.name));
    found
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

/// Whether the lint runs the steps that read only the tree as operators, which the bench and the equivalence test set.
pub(crate) static UNLOWERED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

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
    store: Arc<dyn facts::interface::FactsPort>,
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
    store: Arc<dyn facts::interface::FactsPort>,
    lint_store: Arc<dyn ryo_core::outbound::lint_store::interface::LintStorePort>,
) -> Result<
    (
        Vec<Vec<String>>,
        ryo_core::inbound::use_cases::lint::types::LintReport,
    ),
    String,
> {
    lint_report_hosted(root, rules, store, lint_store, Vec::new(), false)
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
    store: Arc<dyn facts::interface::FactsPort>,
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
    store: Arc<dyn facts::interface::FactsPort>,
    lint_store: Arc<dyn ryo_core::outbound::lint_store::interface::LintStorePort>,
    hosted: Vec<ryo_values::domain::hosted::types::HostRelation>,
    manifests_read: bool,
    additional: Vec<LanguageHandles>,
) -> Result<
    (
        Vec<Vec<String>>,
        ryo_core::inbound::use_cases::lint::types::LintReport,
    ),
    String,
> {
    let rust = ryo_rust::linked();
    let mut handles = vec![LanguageHandles::from(rust)];
    handles.extend(additional);
    // Rules that locate a finding in a manifest read its tree, which only the TOML language gives.
    if manifests_read {
        let toml = ryo_toml::linked();
        handles.push(LanguageHandles::from(toml));
    }
    let compile = CompileUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    );
    let collected = rows::types::Collector::default();
    let kept: Arc<dyn RowsPort> = Arc::new(rows::factory::collect(collected.clone()));
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
        let analysis_digest: Arc<dyn digest::interface::DigestPort> =
            Arc::new(digest::factory::blake3());
        LintUseCaseImpl::new(
            ryo_core::application::file_analysis::service::FileAnalysis::new(
                Arc::clone(&analysis_corpus),
                Arc::clone(&analysis_language),
                Arc::clone(&analysis_digest),
                Arc::clone(&analysis_memory),
            ),
            ryo_core::application::cross_file::service::CrossFile::new(Arc::clone(
                &analysis_corpus,
            )),
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
            Arc::new(scheduler::factory::rayon(scheduler::factory::pool(
                ryo_values::domain::runs::types::WorkersAsked(0),
            ))),
            scheduler::factory::workers(ryo_values::domain::runs::types::WorkersAsked(0)),
            Arc::clone(&kept),
            Arc::clone(&analysis_digest),
            lint_store,
        )
    };
    let use_case = if UNLOWERED.load(std::sync::atomic::Ordering::Relaxed) {
        use_case.unlowered()
    } else {
        use_case
    };
    let use_case = if COUNTING.load(std::sync::atomic::Ordering::Relaxed) {
        use_case.counting()
    } else {
        use_case
    };
    let request = LintRequest {
        answer: ryo_values::domain::runs::types::AnswerShape::Findings,
        hosted,
        session: Session {
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
        id: ryo_values::domain::runs::types::RuleId(each.name.as_str().into()),
        query: ryo_values::domain::program::types::QueryBundle::from(CoreQueryText(
            each.query.as_str().into(),
        )),
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
    let sample = tempfile::tempdir().ok()?;
    std::fs::write(sample.path().join("lib.rs"), "use std::fmt;\nfn run(value: u8) -> u8 { value }\nstruct Point { x: u8 }\nimpl Point { fn new() -> Self { Point { x: 0 } } }\n").ok()?;
    lint_over(sample.path(), vec![rule_of(each)])
        .err()
        .map(|failure| format!("ryo does not run the recorded query: {failure}"))
}

/// The findings a lint kept, each as the text a recording holds: the rule, the subject, the key, the line and the character.
pub(crate) fn base_rows(collected: &rows::types::Collector) -> Vec<Vec<String>> {
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

/// A finding's typed row as the text a recording holds: its rule, its subject, its key values joined with commas, its line and its
/// character, each value as it reads (a span as its two ends); its columns, which a recording does not hold, left out. A row not laid
/// out as a finding's is each of its cells as text.
pub(crate) fn finding_texts(
    cells: &[Option<ryo_values::domain::records::types::Cell>],
) -> Vec<String> {
    use ryo_values::domain::records::types::Cell;
    let text = |cell: &Option<Cell>| match cell {
        Some(Cell::File(each) | Cell::Text(each)) => each.to_string(),
        Some(Cell::Number(number)) => number.0.to_string(),
        Some(Cell::Span(span))
            if ryo_language::domain::relations::service::location(*span)
                == ryo_language::domain::relations::types::Locatedness::Absent =>
        {
            ryo_language::domain::relations::constants::UNLOCATED.to_owned()
        }
        Some(Cell::Span(span)) => format!("{}..{}", span.start.0, span.end.0),
        None => String::new(),
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
            let key: Vec<String> = rest
                .iter()
                .take(usize::try_from(count.0).unwrap_or(0))
                .map(text)
                .collect();
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
