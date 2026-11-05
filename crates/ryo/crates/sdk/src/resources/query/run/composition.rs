//! The wiring of `query run`: the edge takes the parts its caller supplies, chooses each implementation
//! in its builder, and builds the service it calls. `compose` only constructs. Nothing here implements a port.

use ryo_core::inbound::use_cases::lint::interface::LintUseCase;
use ryo_core::outbound::{
    corpus, digest, facts, language, lint_store, matching, memory, query_language, rows,
};
use std::sync::Arc;

use crate::domain::capacity::types::WorkerPool;
use crate::domain::language::types::Language;

/// The dependencies the caller supplies: how the service is built over the adapters its builder chose.
pub(crate) struct QueryRunParts<Construct> {
    /// Builds the service.
    pub service: Construct,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<Service, Construct>(parts: QueryRunParts<Construct>) -> Service
where
    Service: LintUseCase,
    Construct: FnOnce() -> Service,
{
    (parts.service)()
}

/// The production place the rows of a run are kept in, which the caller takes them from after the run.
pub(crate) fn build_rows() -> rows::types::Collector {
    rows::types::Collector::default()
}

/// Chooses the production adapters and hands `compose` the construction of the service over them: the languages that read files,
/// where the rows are kept, and the pool the caller built. The query runs as the one rule of a lint whose answer is its own columns, which keeps nothing: the
/// store is the one in memory, never read or written.
pub(crate) fn build(
    languages: Vec<Language>,
    collector: rows::types::Collector,
    root: ryo_language::inbound::use_cases::project::types::RootPath,
    budget: ryo_values::domain::runs::types::Budget,
    pool: WorkerPool,
) -> impl LintUseCase {
    let workers = pool.workers();
    let pool = pool.of();
    let skipped = corpus::types::SkippedDirectories(
        languages
            .iter()
            .flat_map(|language| language.skips().iter().map(|skip| skip.0.into()))
            .collect(),
    );
    let handles = languages
        .into_iter()
        .map(language::types::LanguageHandles::from)
        .collect();
    let linked = language::factory::use_case(handles);
    compose(QueryRunParts {
        service: move || {
            let syntax = Arc::new(ryo_query_language::outbound::syntax::factory::winnow());
            let catalog = Arc::new(ryo_query_language::outbound::catalog::factory::language());
            let compile = Arc::new(
                ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl::new(
                    syntax, catalog,
                ),
            );
            let typecheck = Arc::new(
                ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl::new(
                    Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
                    Arc::new(ryo_query_language::outbound::catalog::factory::language()),
                ),
            );
            let files: Arc<dyn corpus::interface::CorpusPort> = Arc::new(corpus::factory::ignore(
                root,
                ryo_values::domain::files::types::CorpusListing::Walked,
                Vec::new(),
                skipped,
            ));
            let port: Arc<dyn language::interface::LanguagePort> = Arc::new(linked);
            let digests: Arc<
                dyn ryo_execution::inbound::use_cases::execution::types::digest::DigestPort,
            > = Arc::new(digest::factory::blake3());
            let machine: Arc<dyn memory::interface::MemoryPort> =
                Arc::new(memory::factory::sysinfo(budget));
            ryo_core::inbound::use_cases::lint::use_case::LintUseCaseImpl::new(
                ryo_core::application::file_analysis::service::FileAnalysis::new(
                    Arc::clone(&files),
                    Arc::clone(&port),
                    Arc::clone(&digests),
                    Arc::clone(&machine),
                ),
                ryo_core::application::cross_file::service::CrossFile::new(Arc::clone(&files), std::sync::Arc::new(ryo_core::outbound::publication::factory::execution(std::sync::Arc::new(ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl::new())))),
                Arc::clone(&files),
                Arc::new(facts::factory::memory()),
                port,
                Arc::new(matching::factory::globset()),
                Arc::new(query_language::factory::use_case(compile, typecheck)),
                machine,
                Arc::new(ryo_core::outbound::operators::factory::use_case(ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new())),
std::sync::Arc::new(ryo_core::outbound::derived::factory::use_case(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new())),
                pool,
                workers,
                Arc::new(rows::factory::collect(collector)),
                digests,
                Arc::new(lint_store::factory::memory()),
            )
        },
    })
}
