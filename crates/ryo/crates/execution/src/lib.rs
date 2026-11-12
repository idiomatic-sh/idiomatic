#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]

//! Ryo's sole execution owner; public callers enter through its inbound use case.
pub mod application;
pub mod inbound;
pub mod outbound;

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;

/// The derived access the prepare use case is given, composed from the store's two derived factories and its json route
/// accounting format for the edges above the engine crates: core, the SDK and the command line name execution only, never
/// the store.
#[must_use]
pub fn derived() -> ryo_execution_runtime::application::file_dag::derived::service::ViewAccess {
    ryo_execution_runtime::application::file_dag::derived::service::ViewAccess::routed(
        std::sync::Arc::new(ryo_execution_store::outbound::derived::factory::use_case(
            ryo_execution_store::outbound::derived::factory::semantics(),
        )),
        std::sync::Arc::new(ryo_execution_store::outbound::route_counts::factory::json()),
    )
}
