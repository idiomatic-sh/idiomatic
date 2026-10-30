//! The names of the `packages` concept: where Cargo writes its outputs by default.

/// The directory a workspace's outputs go to, in the workspace's directory.
pub(crate) const OUTPUTS: &str = "target";

/// The file Cargo reads its configuration from, in a `.cargo` directory.
pub(crate) const CONFIGURATION: &str = ".cargo/config.toml";

/// The file Cargo read its configuration from before it was called `config.toml`.
pub(crate) const LEGACY_CONFIGURATION: &str = ".cargo/config";
