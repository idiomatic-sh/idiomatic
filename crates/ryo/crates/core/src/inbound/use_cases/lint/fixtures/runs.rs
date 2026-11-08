//! The runs of `lint` the contract measures: the use case over the corpus, its fakes and its store.

#![expect(
    clippy::expect_used,
    reason = "a contract run over the corpus fails by panicking: the panic is the failure of the check that calls it"
)]

use crate::domain::corpus_copies::types::CopyDigits;
use crate::domain::corpus_copies::types::CopyNaming;
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use crate::outbound::language::interface::LanguagePort;
use crate::outbound::lint_store::interface::LintStorePort;
use crate::outbound::memory::fixtures::fake::MemoryFake;
use crate::outbound::row_sink::fixtures::fake::BytesRowSinkFake;
use crate::outbound::row_sink::interface::RowSinkPort;
use crate::outbound::row_sink;
use crate::outbound::scheduler::fixtures::fake::SchedulerFake;
use crate::outbound::scheduler::interface::SchedulerPort;
use ryo_execution::outbound::facts::interface::FactsPort;
use ryo_execution::outbound::facts;
use ryo_execution::outbound::matching;
use ryo_execution::outbound::query_language;
use ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl;
use ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl;
use ryo_values::domain::files::types::CorpusFile;
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::row_texts::types::Cell as Typed;
use ryo_values::domain::row_texts::types::RowTotal;
use ryo_values::domain::runs::types::AnswerShape;
use std::sync::Arc;
use std::sync::PoisonError;
use std::sync::atomic::Ordering;
use super::super::types::LintRequest;
use super::super::use_case::LintUseCaseImpl;
use super::contract::asking;
use super::contract::rule;
use super::super::interface::LintUseCase as _;
use std::collections::BTreeMap;
use super::corpus::Corpus;
use super::corpus::Stated;
use super::corpus::compared;
use super::corpus::files;
use super::operators::escaped;

/// The compiler of queries the use cases are given.
pub(super) fn compile() -> CompileUseCaseImpl {
    CompileUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    )
}

/// The typechecker of queries the use cases are given.
pub(super) fn typecheck() -> TypecheckUseCaseImpl {
    TypecheckUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    )
}

/// Lints `files` with every rule of the corpus, the scheduler and the facts port given, into `rows`.
pub(super) fn lint(
    corpus: &Corpus,
    scheduler: Arc<dyn SchedulerPort>,
    facts: Arc<dyn FactsPort>,
    files: Vec<CorpusFile>,
    rows: Arc<dyn RowSinkPort>,
) {
    lint_keeping(
        corpus,
        scheduler,
        facts,
        files,
        rows,
        Arc::new(crate::outbound::lint_store::factory::memory()),
    );
}

/// The same lint, keeping its findings in `store`.
pub(super) fn lint_keeping(
    corpus: &Corpus,
    scheduler: Arc<dyn SchedulerPort>,
    facts: Arc<dyn FactsPort>,
    files: Vec<CorpusFile>,
    rows: Arc<dyn RowSinkPort>,
    store: Arc<dyn LintStorePort>,
) {
    let use_case = use_case_over(corpus, scheduler, facts, files, rows, store);
    let rules = corpus.rules.iter();
    let rules =
        rules.map(|(id, (query, governs))| rule(id, query, vec![Glob(governs.as_str().into())]));
    let request = LintRequest {
        source_language: crate::inbound::use_cases::lint::types::SourceProvider::Recognized,
        rules: rules.collect(),
        ..asking(&[])
    };
    let report = use_case.lint(request).expect("the corpus lints");
    assert_eq!(report.unread.len(), 0, "every file of the corpus is read");
}

/// The `lint` use case over `files` and the language of the corpus, with the scheduler and the facts port given, writing into
/// `rows` and keeping its findings in `store`.
pub(super) fn use_case_over(
    corpus: &Corpus,
    scheduler: Arc<dyn SchedulerPort>,
    facts: Arc<dyn FactsPort>,
    files: Vec<CorpusFile>,
    rows: Arc<dyn RowSinkPort>,
    store: Arc<dyn LintStorePort>,
) -> LintUseCaseImpl {
    {
        let analysis_corpus: Arc<dyn crate::outbound::corpus::interface::CorpusPort> =
            Arc::new(CorpusFake {
                files,
                fails: false,
                reads: Arc::default(),
                outside: Vec::new(),
            });
        let analysis_language: Arc<dyn LanguagePort> = Arc::clone(&corpus.language);
        let analysis_memory: Arc<dyn crate::outbound::memory::interface::MemoryPort> =
            Arc::new(MemoryFake::default());
        let analysis_digest: Arc<
            dyn ryo_execution::outbound::digest::interface::DigestPort,
        > = Arc::new(ryo_execution::outbound::digest::fixtures::fake::DigestFake);
        LintUseCaseImpl::new(
            crate::application::file_analysis::service::FileAnalysis::new(
                Arc::clone(&analysis_corpus),
                Arc::clone(&analysis_language),
                Arc::clone(&analysis_digest),
                Arc::clone(&analysis_memory),
            ),
            crate::application::cross_file::service::CrossFile::new(Arc::clone(&analysis_corpus), std::sync::Arc::new(ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl::new())),
            Arc::clone(&analysis_corpus),
            facts,
            Arc::clone(&analysis_language),
            Arc::new(matching::factory::globset()),
            Arc::new(query_language::factory::use_case(
                Arc::new(compile()),
                Arc::new(typecheck()),
            )),
            Arc::clone(&analysis_memory),
            Arc::new(ryo_execution::outbound::operators::factory::use_case(
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())),
            )),
            Arc::new(ryo_execution_store::outbound::derived::factory::use_case(
                ryo_execution_store::outbound::derived::factory::semantics(),
            )),
            scheduler,
            crate::outbound::scheduler::types::WorkerCount(1),
            rows,
            Arc::clone(&analysis_digest),
            store,
        )
    }
}

/// The rows each rule gives over one copy of every input, by the rule's id.
pub(super) fn given(corpus: &Corpus, scheduler: Arc<dyn SchedulerPort>) -> Given {
    let collected = row_sink::factory::CollectorConnection::default();
    let kept: Arc<dyn RowSinkPort> = Arc::new(row_sink::factory::collect(collected.clone()));
    let files = files(corpus, CopyNaming::Input, 1, 1);
    lint(
        corpus,
        scheduler,
        Arc::new(facts::factory::memory()),
        files,
        Arc::clone(&kept),
    );
    let mut given = Given::new();
    for row in
        &std::mem::take(&mut *collected.0.lock().unwrap_or_else(PoisonError::into_inner)).rows
    {
        // A row is the rule, the subject, how many key values and each, then its headed columns, line and column.
        if let [
            Some(Typed::Text(rule)),
            path,
            Some(Typed::Number(count)),
            rest @ ..,
        ] = &*row.0
        {
            let key: Vec<String> = rest
                .iter()
                .take(usize::try_from(count.0).unwrap_or(0))
                .map(|cell| compared(cell.as_ref()))
                .collect();
            record(&mut given, rule, &compared(path.as_ref()), &key.join(","));
        }
    }
    given
}

/// Checks that each rule gave the rows its fixture states.
pub(super) fn agrees(given: &Given, corpus: &Corpus) {
    let wrong = disagreements(given, &corpus.expected);
    assert!(
        wrong.is_empty(),
        "these rules do not give the rows their fixtures state, which are:\n{wrong}"
    );
}

/// What a lint and a query wrote, and what the lint's store holds of its findings once the run is over.
pub(super) struct Written {
    /// The bytes written.
    pub(super) bytes: usize,
    /// The bytes the store holds.
    pub(super) kept: usize,
}

/// Reads `copies` copies of every input, lints them and runs a query over them, and answers how
/// many bytes were written and how many the lint's store holds.
pub(super) fn write_all(corpus: &Corpus, copies: u32) -> Written {
    let linted = Arc::new(BytesRowSinkFake::default());
    let files = files(
        corpus,
        CopyNaming::Copy {
            digits: CopyDigits(5),
        },
        copies,
        1,
    );
    let facts = Arc::new(facts::factory::memory());
    let store: Arc<dyn LintStorePort> = Arc::new(crate::outbound::lint_store::factory::memory());
    lint_keeping(
        corpus,
        Arc::new(SchedulerFake::default()),
        facts,
        files,
        Arc::<BytesRowSinkFake>::clone(&linted),
        Arc::clone(&store),
    );
    // What the store holds is what letting it go gives back.
    let holding = dhat::HeapStats::get().curr_bytes;
    drop(store);
    let kept = holding.saturating_sub(dhat::HeapStats::get().curr_bytes);
    let written = Arc::new(BytesRowSinkFake::default());
    run_query(
        corpus,
        copies,
        1,
        u64::MAX,
        Arc::<BytesRowSinkFake>::clone(&written),
    );
    Written {
        bytes: linted.bytes.load(Ordering::Relaxed) + written.bytes.load(Ordering::Relaxed),
        kept,
    }
}

/// Runs the query over `copies` copies of every input, each as long as `times` of it, writing at most `limit` rows into `written`.
pub(super) fn run_query(
    corpus: &Corpus,
    copies: u32,
    times: usize,
    limit: u64,
    written: Arc<BytesRowSinkFake>,
) {
    let files = files(
        corpus,
        CopyNaming::Copy {
            digits: CopyDigits(5),
        },
        copies,
        times,
    );
    let use_case = use_case_over(
        corpus,
        Arc::new(SchedulerFake::default()),
        Arc::new(facts::factory::memory()),
        files,
        written,
        Arc::new(crate::outbound::lint_store::factory::memory()),
    );
    let text = "let a = select * from \"syntax-tree\"; emit a subject file key(span);";
    let request = LintRequest {
        source_language: crate::inbound::use_cases::lint::types::SourceProvider::Recognized,
        rules: vec![rule("query", text, Vec::new())],
        answer: AnswerShape::Rows(RowTotal(limit)),
        ..asking(&[])
    };
    let report = use_case.lint(request).expect("the corpus runs");
    assert!(report.unread.is_empty(), "every file of the corpus is read");
}

/// The rows the rules gave, one line each, by the rule's id.
type Given = BTreeMap<String, Vec<String>>;

/// Files the row a rule gave, as a line of `path`, a tab and the escaped `key`, under the rule.
fn record(given: &mut Given, rule: &str, path: &str, key: &str) {
    given
        .entry(rule.to_owned())
        .or_default()
        .push(format!("{path}\t{}", escaped(key)));
}

/// The rules whose rows, sorted, are not the rows stated for them, each with the rows it gave; empty when every rule agrees.
fn disagreements(given: &Given, stated: &Stated) -> String {
    let mut wrong = Vec::new();
    for (id, expected) in &stated.0 {
        let mut rows: Vec<&str> = given
            .get(id)
            .map(|lines| lines.iter().map(String::as_str).collect())
            .unwrap_or_default();
        rows.sort_unstable();
        let rows = rows.join("\n");
        if rows != expected.trim_end() {
            wrong.push(format!("== {id}\n{rows}"));
        }
    }
    wrong.join("\n")
}
