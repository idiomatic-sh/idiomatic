//! The identity of an admitted population, assigned only by the engine.

/// Assigned only after application admission; canonical DAG producers keep one ID across rules.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::derived::population::service::PopulationId;
/// fn same(a: PopulationId, b: PopulationId) -> bool {
///     a == b
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PopulationId(pub(crate) usize);
