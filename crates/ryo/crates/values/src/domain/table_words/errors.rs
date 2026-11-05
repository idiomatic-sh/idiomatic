//! Encoded publication refusals.
/// A demanded publication cannot be certified from a shortened/malformed encoded frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum WordsDecodeError {
    #[error("encoded publication is truncated, noncanonical or has incompatible frame schemas")]
    Incomplete,
}
