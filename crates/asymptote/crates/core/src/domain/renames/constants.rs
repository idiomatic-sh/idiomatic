//! The shape of an operation file's name.

/// The extension of an operation's file, after the dot that ends its stem.
pub const EXTENSION: &str = "lock";

/// What ends an operation file's stem.
pub const EXTENSION_MARK: char = '.';
/// How many hex digits an operation's id has.
pub const ID_DIGITS: usize = 64;
