//! The names of the `paths` concept: the text its service cuts paths with.

/// What separates the parts of a path.
pub(crate) const SEPARATOR: &str = "/";

/// What a path may be cut at: the separator, and the one Windows writes.
pub(crate) const CUTS: [char; 2] = ['/', '\\'];

/// The part of a path that stays where it is.
pub(crate) const HERE: &str = ".";

/// The part of a path that goes to the directory above.
pub(crate) const UP: &str = "..";

/// The file name of a manifest.
pub(crate) const MANIFEST: &str = "Cargo.toml";

/// The file name that owns the directory it is in as a module.
pub(crate) const DIRECTORY_OWNER: &str = "mod.rs";
