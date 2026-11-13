//! The names of the `terms` concept: where the numbers of the words start.

use super::types::NameText;

/// The first number a word has. Names a file writes are numbered from zero and are fewer than this.
pub(crate) const TERM_BASE: u32 = 0xFFFF_0000;

/// How many numbers each block has.
pub(crate) const BLOCK_SIZE: u32 = 0x100;

/// What the words of `DeclarationKind` are the words of, as the catalog's listings name it.
pub(crate) const DECLARATION_OWNER: NameText<'static> = NameText("defines");

/// What the words of `ParameterKind` are the words of.
pub(crate) const PARAMETER_OWNER: NameText<'static> = NameText("parameter");

/// What the words of `ValueKind` are the words of.
pub(crate) const VALUE_OWNER: NameText<'static> = NameText("entry and sequence-element");

/// What the words of `MappingKind` are the words of.
pub(crate) const MAPPING_OWNER: NameText<'static> = NameText("mapping");

/// What the words of `DependencyKind` are the words of.
pub(crate) const DEPENDENCY_OWNER: NameText<'static> = NameText("package-dependency");

/// What the words of `DependencyOrigin` are the words of.
pub(crate) const ORIGIN_OWNER: NameText<'static> = DEPENDENCY_OWNER;

/// What the reasons for an unknown value are the words of.
pub(crate) const UNKNOWN_OWNER: NameText<'static> = NameText("an unknown value");
