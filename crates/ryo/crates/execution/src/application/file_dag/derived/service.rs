//! The derived access the prepare use case is given, composed from the store's two derived factories and its json route
//! accounting format for the edges above the engine crates: core, the SDK and the command line name execution only, never
//! the store.

/// The derived capability a prepared program holds; an edge hands it to the use case, so callers outside the engine crates
/// never name the store.
pub use ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess;

/// The derived access the prepare use case is given, composed from the store's two derived factories and its json route
/// accounting format.
///
/// # Examples
///
/// ```
/// let _access = ryo_execution::application::file_dag::derived::service::derived();
/// ```
#[must_use]
pub fn derived() -> DerivedAccess {
    DerivedAccess::from(std::sync::Arc::new(
        ryo_execution_store::outbound::derived::factory::use_case(
            ryo_execution_store::outbound::derived::factory::semantics(),
        ),
    ))
    .routed(std::sync::Arc::new(
        ryo_execution_store::outbound::route_counts::factory::json(),
    ))
}
