//! Encoded publication refusals.
/// A demanded publication cannot be certified from a shortened/malformed encoded frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TableWordsError {
    /// The words are truncated, not in canonical form, or their frames disagree on their schema.
    #[error("encoded publication is truncated, noncanonical or has incompatible frame schemas")]
    Incomplete,
}
