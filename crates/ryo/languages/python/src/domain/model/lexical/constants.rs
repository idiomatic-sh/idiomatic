//! The constants of the model's `lexical` part.

use ryo::languages::{NameText, Position};

/// The module: the root of the file's nodes, and its outermost scope.
pub(crate) const MODULE: Position = Position(0);

/// The position of no node.
pub(crate) const NO_NODE: Position = Position(u32::MAX);

/// What a wildcard import names in place of a name.
pub(crate) const WILDCARD: NameText<'static> = NameText("*");
