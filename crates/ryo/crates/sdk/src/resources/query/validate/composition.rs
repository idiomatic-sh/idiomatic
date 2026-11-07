//! The wiring of `query validate`: the edge takes the parts its caller supplies, chooses each
//! implementation in its builder, and builds the service it calls. It only constructs: the verb opens no client, so it
//! has no connections. Nothing here implements a port.

use std::sync::Arc;

use ryo_core::inbound::use_cases::validate::interface::ValidateUseCase;
use ryo_core::outbound::language;
use ryo_execution::outbound::query_language;

use crate::domain::language::types::Language;

/// The dependencies the caller supplies: how the service is built over the adapters its builder chose.
pub(crate) struct QueryValidateParts<Construct> {
    /// Builds the service.
    pub service: Construct,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<Service, Construct>(parts: QueryValidateParts<Construct>) -> Service
where
    Service: ValidateUseCase,
    Construct: FnOnce() -> Service,
{
    (parts.service)()
}

/// Chooses the production adapters and hands `compose` the construction of the service over them: the query language's own syntax
/// and catalog, which check and compile the text, and the languages linked, whose kinds the words a query writes for a kind are
/// checked against.
pub(crate) fn build(languages: Vec<Language>) -> impl ValidateUseCase {
    let handles = languages
        .into_iter()
        .map(language::types::LanguageHandles::from)
        .collect();
    let language = language::factory::use_case(handles);
    compose(QueryValidateParts {
        service: move || {
            let compile = Arc::new(
                ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl::new(
                    Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
                    Arc::new(ryo_query_language::outbound::catalog::factory::language()),
                ),
            );
            let typecheck = Arc::new(
                ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl::new(
                    Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
                    Arc::new(ryo_query_language::outbound::catalog::factory::language()),
                ),
            );
            ryo_core::inbound::use_cases::validate::use_case::ValidateUseCaseImpl::new(
                Arc::new(query_language::factory::use_case(compile, typecheck)),
                Arc::new(language),
            )
        },
    })
}
