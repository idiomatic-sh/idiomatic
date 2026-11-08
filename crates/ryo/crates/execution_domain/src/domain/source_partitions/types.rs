//! Checked declarations retain their private source provenance.
pub mod source_partitions;

/// Number of immutable hosted relation snapshots.
pub(crate) struct HostedCount(pub usize);
/// Exact declaration invariant that failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PartitionRefusal {
    BindingOrdinal,
    MissingDeclaration,
    UndeclaredInput,
    MissingConsumer,
    ConsumerMismatch,
    MissingQuery,
    MissingRule,
    MissingOutput,
    OutputMismatch,
    MissingStep,
    IndeterminatePhase,
    MissingBinding,
    PhaseMismatch,
}
