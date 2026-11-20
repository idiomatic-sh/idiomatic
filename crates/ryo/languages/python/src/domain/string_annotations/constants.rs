//! The names of the `string_annotations` concept: the modules whose names can name a typing form, by what the form reads.

/// The modules whose names include the forms that read a call's argument or an annotated value as a type (`cast`, `NewType`,
/// `TypeVar`, `TypeAlias`) and the forms that read a subscription's operands (`Literal`, `Annotated`, the generic aliases).
pub(crate) const TYPING_MODULES: [&str; 2] = ["typing", "typing_extensions"];

/// The other modules whose names include generic classes: a subscription of one reads its operands as types.
pub(crate) const GENERIC_MODULES: [&str; 4] = ["collections", "contextlib", "re", "weakref"];

/// The builtin classes whose subscription reads its operands as types.
pub(crate) const GENERIC_BUILTINS: [&str; 6] =
    ["dict", "frozenset", "list", "set", "tuple", "type"];
