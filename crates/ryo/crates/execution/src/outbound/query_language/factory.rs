//! Builds the `query_language` adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

use std::sync::Arc;

use ryo_query_language::inbound::use_cases::compile::interface::CompileUseCase;
use ryo_query_language::inbound::use_cases::typecheck::interface::TypecheckUseCase;

/// The `use_case` `query_language` adapter, over the `query_language` crate's `compile` and `typecheck` use cases.
#[must_use]
pub fn use_case(
    compile: Arc<dyn CompileUseCase>,
    typecheck: Arc<dyn TypecheckUseCase>,
) -> impl crate::outbound::query_language::interface::QueryLanguagePort {
    super::adapters::use_case::adapter::QueryLanguageUseCaseAdapter::new(compile, typecheck)
}
