//! The segments a path is written with.

/// The separator of a path's segments.
pub const SEPARATOR: char = '/';
/// The segment that names the directory itself.
pub const HERE: &str = ".";
/// The segment that names the parent directory.
pub const PARENT: &str = "..";
/// The fingerprint of a violation about a whole file, not a place in it: no content of the file changes it, so the
/// ratchet matches it only at its own path. A fixed value, so records of it match across runs.
pub const WHOLE_FILE: super::types::Fingerprint = super::types::Fingerprint(0xb38a_6c34_8fd3_539f);
/// The level of a violation that is a note: the word SARIF, the lock and the JSON answer write.
pub const NOTE: &str = "note";
/// The level of a violation that is a warning.
pub const WARNING: &str = "warning";
/// The level of a violation that is an error.
pub const ERROR: &str = "error";
