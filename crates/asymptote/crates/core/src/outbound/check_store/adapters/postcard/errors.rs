//! The `postcard` adapter's failures.

/// Why an operation of `postcard` failed: the store's bytes are no store of this build, each a cold miss, which the
/// adapter loads as nothing kept and writes anew.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CheckStorePostcardError {
    /// The bytes do not decode as a store: it is rebuilt, never migrated.
    #[error("the bytes do not decode as a store: {0}")]
    Undecodable(postcard::Error),
    /// Another build wrote the store.
    #[error("another build wrote the store")]
    OtherBuild,
    /// A row names a text the store's strings table does not hold.
    #[error("a row names a text the store does not hold")]
    Unnamed,
    /// The diagnostic extension after the store does not decode.
    #[error("the diagnostic extension does not decode: {0}")]
    Extension(postcard::Error),
    /// The diagnostic extension decodes but does not match the store's runs, or bytes follow it.
    #[error("the diagnostic extension does not match the store")]
    Damaged,
}
