//! The wiring of `query validate`: the edge takes the parts its caller supplies and
//! builds the service it calls. It only constructs. Nothing here implements a port.

use ryo_core::inbound::use_cases::validate::interface::ValidateUseCase;

/// The dependencies the caller supplies: the service the command calls.
pub(crate) struct QueryValidateParts<S> {
    /// The service the command calls.
    pub service: S,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<S>(parts: QueryValidateParts<S>) -> S
where
    S: ValidateUseCase,
{
    parts.service
}

/// Selects the production service: the query language's own syntax and catalog, which check and compile the text, and the languages
/// linked, whose kinds the words a query writes for a kind are checked against.
pub(crate) fn build() -> impl ValidateUseCase {
    use std::sync::Arc;

    use ryo_core::inbound::use_cases::validate::use_case::ValidateUseCaseImpl;
    use ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl;
    use ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl;

    let compile = Arc::new(CompileUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    ));
    let typecheck = Arc::new(TypecheckUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    ));
    compose(QueryValidateParts {
        service: ValidateUseCaseImpl::new(
            Arc::new(ryo_execution::outbound::query_language::factory::use_case(
                compile, typecheck,
            )),
            Arc::new(crate::languages::linked()),
        ),
    })
}
