//! The text the macros expand to in the crate that calls them.

/// The version of the crate that links a language: `env!` is read where the macro expands, which is the language's own crate.
#[macro_export]
#[doc(hidden)]
macro_rules! language_crate_version {
    () => {
        env!("CARGO_PKG_VERSION")
    };
}
