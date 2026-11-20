//! Why `load` refuses.

/// Why a rule set could not be loaded: only a pack that is not this rule set's. A rule that cannot be loaded is not an error:
/// `load` answers it among the refused.
///
/// # Examples
///
/// ```
/// use ryo_engine::inbound::use_cases::load::errors::LoadError;
///
/// assert_eq!(LoadError::PackCorrupt.to_string(), "pack bytes do not decode");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LoadError {
    /// The pack was written by another engine version.
    #[error("pack written by another engine version")]
    PackVersion,
    /// The pack was written for other rules.
    #[error("pack written for other rules")]
    PackDigest,
    /// The pack's bytes do not decode.
    #[error("pack bytes do not decode")]
    PackCorrupt,
}
