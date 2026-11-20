//! Why `walk` refuses.

/// Why a file was not walked.
///
/// # Examples
///
/// ```
/// use ryo_language::inbound::use_cases::walk::errors::WalkError;
///
/// assert_eq!(WalkError::Library.to_string(), "the parser failed");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum WalkError {
    /// The language does not read this file.
    #[error("the language does not read this file")]
    Unsupported,
    /// The parser library failed.
    #[error("the parser failed")]
    Library,
}
