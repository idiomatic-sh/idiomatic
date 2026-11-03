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
    use ryo_core::outbound::digest::interface::DigestPort;
    use ryo_core::outbound::language::interface::LanguagePort;
    use ryo_core::outbound::lint_store::interface::LintStorePort;
    use ryo_core::outbound::memory::interface::MemoryPort;
    use ryo_core::outbound::{
        corpus, digest, facts, lint_store, matching, memory, query_language, scheduler,
    };
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
    compose(QueryRunParts {
        service: LintUseCaseImpl::new(
            FileAnalysis::new(
                Arc::clone(&files),
                Arc::clone(&language),
                Arc::clone(&digests),
                Arc::clone(&memory),
            ),
            CrossFile::new(Arc::clone(&files)),
            Arc::clone(&files),
            Arc::new(facts::factory::memory()),
            language,
            Arc::new(matching::factory::globset()),
            Arc::new(query_language::factory::use_case(compile, typecheck)),
            memory,
            Arc::new(ryo_core::outbound::operators::factory::use_case(
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
            )),
            Arc::new(scheduler::factory::rayon(scheduler::factory::pool(workers))),
            scheduler::factory::workers(workers),
            Arc::new(crate::output::schemas::rows(out, edge.format)),
            digests,
            store,
        ),
    })
}
