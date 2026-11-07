//! The wiring of `query run`: the edge takes the parts its caller supplies and
//! builds the service it calls. It only constructs. Nothing here implements a port.

use ryo_core::inbound::use_cases::lint::interface::LintUseCase;

/// The dependencies the caller supplies: the service the command calls.
pub(crate) struct QueryRunParts<S> {
    /// The service the command calls.
    pub service: S,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<S>(parts: QueryRunParts<S>) -> S
where
    S: LintUseCase,
{
    parts.service
}

/// Selects the production adapters of the service: the rows go to `out` as `format` prints them, and the command line links the languages it was
/// built with. The queries run as the rules of a lint whose answer is their own columns, which keeps nothing: the store, in the
/// cache directory the edge names, is never read or written.
pub(crate) fn build(
    out: Box<dyn std::io::Write + Send>,
    edge: &crate::edge::Edge,
    kept_in: ryo_core::outbound::lint_store::types::Store,
    root: ryo_language::inbound::use_cases::project::types::RootPath,
    budget: ryo_values::domain::runs::types::Budget,
    workers: ryo_values::domain::runs::types::WorkersAsked,
) -> impl LintUseCase {
    use ryo_core::application::cross_file::service::CrossFile;
    use ryo_core::application::file_analysis::service::FileAnalysis;
    use ryo_core::inbound::use_cases::lint::use_case::LintUseCaseImpl;
    use ryo_core::outbound::corpus::interface::CorpusPort;
    use ryo_core::outbound::language::interface::LanguagePort;
    use ryo_core::outbound::lint_store::interface::LintStorePort;
    use ryo_core::outbound::memory::interface::MemoryPort;
    use ryo_core::outbound::{corpus, lint_store, memory, scheduler};
    use ryo_execution::outbound::{digest, facts, matching, query_language};
    use ryo_execution::inbound::use_cases::execution::types::digest::DigestPort;
    use ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl;
    use ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl;
    use std::sync::Arc;

    let compile = Arc::new(CompileUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    ));
    let typecheck = Arc::new(TypecheckUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    ));
    let files: Arc<dyn CorpusPort> = Arc::new(corpus::factory::ignore(
        root,
        ryo_values::domain::files::types::CorpusListing::Walked,
        Vec::new(),
        {
            let mut skipped = crate::languages::skipped();
            skipped.0.push(edge.store.0.clone());
            skipped
        },
    ));
    let language: Arc<dyn LanguagePort> = Arc::new(crate::languages::linked());
    let digests: Arc<dyn DigestPort> = Arc::new(digest::factory::blake3());
    let memory: Arc<dyn MemoryPort> = Arc::new(memory::factory::sysinfo(budget));
    let store: Arc<dyn LintStorePort> = Arc::new(lint_store::factory::postcard(kept_in));
    let started = scheduler::types::WorkerCount(
        ryo_values::domain::runs::service::workers_to_start(
            workers,
            ryo_values::domain::runs::types::MachineWorkers(
                u32::try_from(std::thread::available_parallelism().map_or(1, std::num::NonZero::get)).unwrap_or(1),
            ),
        )
        .0,
    );
    compose(QueryRunParts {
        service: LintUseCaseImpl::new(
            FileAnalysis::new(
                Arc::clone(&files),
                Arc::clone(&language),
                Arc::clone(&digests),
                Arc::clone(&memory),
            ),
            CrossFile::new(Arc::clone(&files), std::sync::Arc::new(ryo_core::outbound::publication::factory::execution(std::sync::Arc::new(ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl::new())))),
            Arc::clone(&files),
            Arc::new(facts::factory::memory()),
            language,
            Arc::new(matching::factory::globset()),
            Arc::new(query_language::factory::use_case(compile, typecheck)),
            memory,
            Arc::new(ryo_execution::outbound::operators::factory::use_case(
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())),
            )),
std::sync::Arc::new(ryo_execution::outbound::derived::factory::use_case(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new())),
            Arc::new(scheduler::factory::rayon(started)),
            started,
            Arc::new(crate::output::schemas::rows(out, edge.format)),
            digests,
            store,
        ),
    })
}
