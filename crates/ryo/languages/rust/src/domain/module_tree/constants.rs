//! The constants of the `module_tree` concept: the text its service and the readers of module paths look for.

/// The file name that owns the directory it is in as a module.
pub(crate) const DIRECTORY_OWNER: &str = crate::domain::paths::constants::DIRECTORY_OWNER;

/// The first segment of a path written from the root of the crate.
pub(crate) const CRATE: &str = "crate";

/// The first segment of a path written from the module itself.
pub(crate) const SELF: &str = "self";

/// The first segment of a path written from the module above.
pub(crate) const SUPER: &str = "super";

/// The first segment of a path a macro writes from the crate that defined it.
pub(crate) const MACRO_CRATE: &str = "$crate";

/// The first segment of a path written from the type being implemented.
pub(crate) const SELF_TYPE: &str = "Self";

/// The last segment of a path that names everything in a module.
pub(crate) const GLOB: &str = "*";

/// What separates the segments of a module path.
pub(crate) const MODULE_SEPARATOR: &str = "::";

/// The extension of a Rust source file.
pub(crate) const SOURCE: &str = ".rs";
