//! What every replay test shares: the lint over a tree it writes, with the real languages and ports, and a finding as text.
//! Each suite that lints with the real engine includes it whole.

#[path = "rules.rs"]
pub(crate) mod rules;

use ryo_core::inbound::use_cases::lint::interface::LintUseCase as _;
use std::path::Path;
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
use ryo_execution_runtime::outbound::digest;
use ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl;
use ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl;
use ryo_values::domain::plan_parts::types::{FlagState, LiteralText, ParameterName, WholeNumber};

/// What a parameter of a rule is bound to, when ryo can bind it.
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

/// The languages the replay links: the ones `ryo lint` links in this build (`ryo_cli::linked_connections`), so a rule
/// answers what a user's run answers. The hosted lint and the counting lint both link these.
pub(crate) fn replay_languages() -> Vec<LanguageConnection> {
    ryo_cli::linked_connections()
}

/// The edge `ryo` builds for a command over `root`: the store's directory and the directories a walk skips are the command's own,
/// so the replay reads the files `ryo` reads and keeps its store where `ryo` does.
pub(crate) fn command_edge(root: &Path) -> ryo_cli::edge::Edge {
    ryo_cli::resources::edge_at(
        RootPath(root.to_string_lossy().into()),
        RootPath(".".into()),
        ryo_values::domain::runs::types::MachineWorkers(1),
        ryo_cli::output::Format::Quiet,
    )
}

/// The real store, kept in the cache directory under `root`, for the build the tests are.
pub(crate) fn store_at(
    root: &Path,
) -> Arc<dyn ryo_core::outbound::lint_store::interface::LintStorePort> {
    Arc::new(ryo_core::outbound::lint_store::factory::postcard(
        ryo_core::outbound::lint_store::types::Store {
            directory: ryo_core::outbound::lint_store::types::CacheDirectory(
                root.join(&*command_edge(root).store.0)
                    .to_string_lossy()
                    .into(),
            ),
            build: ryo_core::outbound::lint_store::types::BuildIdentity("replay".into()),
        },
    ))
}

/// Whether the lint counts the rows each step is fed, which a test of what a run governs sets.
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
    // Rules that locate a finding in a manifest read its tree, which only the TOML language gives: those runs link the
    // replay's languages; the others the Rust language alone. A scene's own providers come after.
    let mut handles = if manifests_read {
        replay_languages()
    } else {
        vec![LanguageConnection::from(ryo_rust::linked())]
    };
    handles.extend(additional);
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
                // The directories `ryo` itself skips: the languages' own and the store's.
                command_edge(root).skipped,
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
                ryo_operators::inbound::use_cases::plan::use_case::PlanUseCaseImpl::new(), ryo_operators::inbound::use_cases::run::use_case::RunUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())))),
            Arc::new(scheduler::factory::rayon(started)),
            started,
            Arc::clone(&kept),
            Arc::clone(&analysis_digest),
            lint_store,
        )
    };
    let use_case = if COUNTING.load(std::sync::atomic::Ordering::Relaxed) {
        use_case.counting(|| {
            std::sync::Arc::new(ryo_execution_runtime::outbound::observer::factory::counting())
        })
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
        .map_err(|error| format!("the rules do not run: {error}"))?;
    let rows = base_rows(&collected);
    Ok((rows, report))
}

/// The findings a lint kept, each as text: the rule, the subject, the key, the line and the character.
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

/// A finding's typed row as text: its rule, its subject, its key values joined with commas, its line and its
/// character, each value as it reads (a span as its two ends); its columns left out. A row not laid
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
