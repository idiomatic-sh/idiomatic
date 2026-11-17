//! The words a pathspec and the digest of what it names are written with.

/// What separates a path's segments.
pub const SEPARATOR: char = '/';
/// What a pathspec relative to the product's root may start with.
pub const CURRENT: &str = "./";
/// The segment that names the directory itself.
pub const ROOT: &str = ".";
/// The segment that names the parent directory.
pub const PARENT: &str = crate::domain::violations::constants::PARENT;
/// The wildcard that matches any run of characters.
pub const ANY: char = '*';
/// The wildcard that matches any one character.
pub const ONE: char = '?';
/// The path of the repository's top, and the stamp of a file with none.
pub const TOP: &str = "";
