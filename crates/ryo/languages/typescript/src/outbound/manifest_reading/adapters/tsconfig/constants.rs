//! The manifest file the tsconfig adapter reads.

/// The file name of a TypeScript project configuration.
pub(super) const MANIFEST: &str = "tsconfig.json";

/// The ending of a project configuration's path, which `extends` may leave out.
pub(super) const CONFIGURATION_EXTENSION: &str = ".json";

/// What starts a path relative to the configuration, as `extends` writes it: `./base` or `../base`.
pub(super) const CURRENT: &str = crate::domain::packages::constants::CURRENT;
