//! The types of the `use_case` adapter of the `query_language` port: plain data, public fields and derives, with no constructor and no
//! inherent impl.

use std::sync::Arc;

use ryo_query_language::inbound::use_cases::compile::interface::CompileUseCase;
use ryo_query_language::inbound::use_cases::typecheck::interface::TypecheckUseCase;

/// The `compile` use case of the `query_language` crate, which the adapter calls.
#[derive(Clone)]
pub(crate) struct CompileCalled(pub Arc<dyn CompileUseCase>);

impl std::fmt::Debug for CompileCalled {
    fn fmt(&self, _formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

/// The `typecheck` use case of the `query_language` crate, which the adapter calls.
#[derive(Clone)]
pub(crate) struct TypecheckCalled(pub Arc<dyn TypecheckUseCase>);

impl std::fmt::Debug for TypecheckCalled {
    fn fmt(&self, _formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}
