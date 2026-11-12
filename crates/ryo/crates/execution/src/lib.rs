#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]

//! Ryo's sole execution owner; public callers enter through its inbound use case.
pub mod application;
pub mod inbound;
pub mod outbound;

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;

/// The derived views the prepare use case prepares over, composed from the store's two derived factories and its json
/// route accounting format for the edges above the engine crates: core, the SDK and the command line name execution only,
/// never the store.
#[must_use]
pub fn derived() -> inbound::use_cases::prepare::types::DerivedViews {
    inbound::use_cases::prepare::types::DerivedViews::from(
        application::derived::service::DerivedPreparation::routed(
            std::sync::Arc::new(ryo_execution_store::outbound::derived::factory::use_case(
                ryo_execution_store::outbound::derived::factory::semantics(),
            )),
            std::sync::Arc::new(ryo_execution_store::outbound::route_counts::factory::json()),
        ),
    )
}
