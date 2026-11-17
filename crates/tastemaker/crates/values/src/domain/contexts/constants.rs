//! The names a template's content reads its maps by.

/// `captures.<name>`.
pub const CAPTURES: &str = "captures";
/// `present.<entry>`.
pub const PRESENT: &str = "present";
/// `instances.<spec>`.
pub const INSTANCES: &str = "instances";
/// What joins the words of a param's kebab-case name.
pub const KEBAB_JOIN: char = '-';
/// What joins them in the identifier a template's content reads the param by.
pub const IDENTIFIER_JOIN: &str = "_";
/// An instance's directory name.
pub const NAME: &str = "name";
/// An instance's path from the file's instance.
pub const INSTANCE_PATH: &str = "path";
/// `scaffold`: set when a file is being written for the first time, never when it is checked.
pub const SCAFFOLD: &str = "scaffold";
/// What `scaffold` is given while a file is being written.
pub const SCAFFOLDING: &str = "yes";
/// The `--set` name of the extension a relation fixture's `input.*` is given.
pub const EXTENSION: &str = "ext";
