//! The names of the parse contract.
use ryo::languages;

/// The optional capabilities of the project's relations the python language answers; those of its files' trees are what its
/// `Semantics` answers.
pub(crate) const CAPABILITIES: &[languages::OptionalCapability] =
    &[languages::OptionalCapability::ModuleIdentity];
/// The extension of a stub file.
pub(crate) const STUB_EXTENSION: &str = "pyi";
