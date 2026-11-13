//! The flags and refusals of a verification run's arguments.
/// The flag that names the shard a run covers, as `INDEX/COUNT`.
pub(super) const SHARD: &str = "--shard=";
/// The separator between a shard's index and its count.
pub(super) const SHARD_SEPARATOR: char = '/';
/// The refusal for a receipt destination that is empty or given twice.
pub(super) const CASE_RECEIPT_DESTINATION_MUST_BE_N: &str =
    "case receipt destination must be nonempty and supplied once";
/// The refusal for a second shard flag.
pub(super) const DUPLICATE_CASE_SHARD_ARGUMENT: &str = "duplicate case shard argument";
/// The refusal for a flag no verification run reads.
pub(super) const UNEXPECTED_VERIFY_CASES_ARGUMENT: &str = "unexpected verify-cases argument";
/// The refusal for a shard flag that is not `INDEX/COUNT`.
pub(super) const EXPECTED_SHARD_INDEX_COUNT: &str = "expected --shard=INDEX/COUNT";
/// The refusal for a shard index that is not a number.
pub(super) const INVALID_CASE_SHARD_INDEX: &str = "invalid case shard index";
/// The refusal for a shard count that is not a number.
pub(super) const INVALID_CASE_SHARD_COUNT: &str = "invalid case shard count";
