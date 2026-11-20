//! Why a pack does not read back.

/// Why a pack's bytes give no programs.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::packs::errors::PacksError;
///
/// assert_eq!(PacksError::Digest.to_string(), "pack written for other rules");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PacksError {
    /// The pack was written by another engine version.
    #[error("pack written by another engine version")]
    Version,
    /// The pack was written for other rules.
    #[error("pack written for other rules")]
    Digest,
    /// The pack's bytes do not decode.
    #[error("pack bytes do not decode")]
    Corrupt,
}
